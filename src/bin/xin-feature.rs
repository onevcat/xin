use clap::Parser;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "xin-feature",
    about = "Run xin feature tests against real servers"
)]
struct Cli {
    /// Path to a case YAML file.
    #[arg(long, conflicts_with_all = ["case_dir", "all"])]
    case: Option<PathBuf>,

    /// Directory containing cases (defaults to tests/feature/stalwart/cases).
    #[arg(long)]
    case_dir: Option<PathBuf>,

    /// Run all *.yaml cases in --case-dir.
    #[arg(long, requires = "case_dir", conflicts_with = "case")]
    all: bool,

    /// Directory containing the Stalwart docker setup (tests/feature/stalwart).
    #[arg(long, default_value = "tests/feature/stalwart")]
    stalwart_dir: PathBuf,

    /// Reset the docker server state before running (down + rm -rf .state + up).
    #[arg(long)]
    fresh: bool,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,

    /// Human-readable BDD-style description (like `it(...)`).
    #[serde(default)]
    it: Option<String>,

    #[serde(default, rename = "requiresFresh")]
    requires_fresh: bool,

    #[serde(default)]
    seed: Option<Seed>,

    #[serde(default)]
    env: BTreeMap<String, String>,

    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Seed {
    /// Domain to create (e.g. example.org)
    domain: String,

    /// User principals to create.
    users: Vec<SeedUser>,

    /// Optional SMTP injections (fixtures).
    #[serde(default, rename = "smtpInject")]
    smtp_inject: Vec<SmtpInject>,
}

#[derive(Debug, Deserialize)]
struct SeedUser {
    user: String,
    pass: String,

    /// Optional explicit email address. Default: {user}@{domain}
    #[serde(default)]
    email: Option<String>,

    #[serde(default)]
    roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SmtpInject {
    #[serde(rename = "authUser")]
    auth_user: String,

    #[serde(rename = "authPass")]
    auth_pass: String,

    #[serde(rename = "mailFrom")]
    mail_from: String,

    #[serde(rename = "rcptTo")]
    rcpt_to: Vec<String>,

    #[serde(rename = "emlFile")]
    eml_file: String,
}

#[derive(Debug, Deserialize)]
struct Step {
    #[serde(default)]
    name: Option<String>,

    /// Extra human-readable lines to print in runner output for this step (BDD-style).
    #[serde(default)]
    say: Vec<String>,

    #[serde(default)]
    env: BTreeMap<String, String>,

    xin: XinStep,

    #[serde(default)]
    expect: Vec<Assertion>,

    #[serde(default)]
    save: BTreeMap<String, String>,

    #[serde(default)]
    retry: Option<Retry>,

    /// Whether this step expects ok=true. Default: true.
    #[serde(default = "default_expect_ok", rename = "expectOk")]
    expect_ok: bool,
}

fn default_expect_ok() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Retry {
    #[serde(default = "default_retry_attempts")]
    attempts: usize,

    #[serde(default, rename = "sleepMs")]
    sleep_ms: Option<u64>,
}

fn default_retry_attempts() -> usize {
    20
}

#[derive(Debug, Deserialize)]
struct XinStep {
    args: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Assertion {
    /// Optional human-readable label for nicer failure output.
    #[serde(default)]
    label: Option<String>,

    /// JSON pointer (e.g. /ok, /data/items/0/emailId)
    path: String,

    /// Assert equals to this literal.
    #[serde(default)]
    equals: Option<serde_yaml::Value>,

    /// Assert the string value contains this substring.
    #[serde(default)]
    contains: Option<String>,

    /// Assert the value exists and is not null.
    #[serde(default)]
    exists: Option<bool>,
}

#[derive(Debug)]
struct Context {
    case_id: String,
    run_id: String,
    vars: HashMap<String, String>,
}

fn main() {
    let cli = Cli::parse();
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let stalwart_dir = root_dir.join(&cli.stalwart_dir);

    let cases = resolve_cases(&root_dir, &cli).unwrap_or_else(|e| fatal(&e));

    // Build once for all cases.
    run_cmd(
        Command::new("cargo")
            .current_dir(&root_dir)
            .arg("build")
            .arg("-q"),
    )
    .unwrap_or_else(|e| fatal(&format!("cargo build failed: {e}")));

    let xin_bin = root_dir.join("target/debug/xin");

    let mut failed: Vec<String> = Vec::new();

    for case_path in cases {
        let case = read_case(&case_path).unwrap_or_else(|e| {
            fatal(&format!("failed to read case {}: {e}", case_path.display()))
        });

        let file = case_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("(case)");
        if let Some(it) = &case.it {
            eprintln!(
                "\n{}",
                colorize(&format!("=== CASE: {} — {} ===", file, it), "95")
            );
        } else {
            eprintln!("\n{}", colorize(&format!("=== CASE: {} ===", file), "95"));
        }

        let fresh = cli.fresh || case.requires_fresh;

        ensure_stalwart(&stalwart_dir, fresh)
            .unwrap_or_else(|e| fatal(&format!("stalwart setup failed: {e}")));

        if let Some(seed) = &case.seed {
            seed_stalwart(&stalwart_dir, seed)
                .unwrap_or_else(|e| fatal(&format!("stalwart seed failed: {e}")));
        } else {
            // Back-compat: if no seed block is provided, run the legacy seed script.
            let seed_script = stalwart_dir.join("scripts/seed.sh");
            run_cmd(Command::new(&seed_script).current_dir(&stalwart_dir))
                .unwrap_or_else(|e| fatal(&format!("seed.sh failed: {e}")));
        }

        match run_case(&xin_bin, &case) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("FAILED: {}\n{e}", case.id);
                failed.push(case.id);
            }
        }
    }

    if failed.is_empty() {
        eprintln!("\n{}", colorize("OK: all cases passed", "92"));
        return;
    }

    eprintln!("\nFAIL: {} case(s) failed:", failed.len());
    for id in failed {
        eprintln!("- {id}");
    }

    std::process::exit(1);
}

fn use_color() -> bool {
    // Respect NO_COLOR. By default, only emit ANSI escapes when stderr is a terminal.
    // If you really want colors in non-tty logs, set XIN_FEATURE_FORCE_COLOR=1.
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    if std::env::var_os("XIN_FEATURE_FORCE_COLOR").is_some()
        || std::env::var_os("CLICOLOR_FORCE").is_some()
        || std::env::var_os("FORCE_COLOR").is_some()
    {
        return true;
    }

    std::io::stderr().is_terminal()
}

fn colorize(s: &str, code: &str) -> String {
    if use_color() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn resolve_cases(root_dir: &Path, cli: &Cli) -> Result<Vec<PathBuf>, String> {
    if let Some(case) = &cli.case {
        let case_path = if case.is_absolute() {
            case.clone()
        } else {
            root_dir.join(case)
        };
        return Ok(vec![case_path]);
    }

    if cli.all {
        let dir = cli
            .case_dir
            .as_ref()
            .ok_or_else(|| "--all requires --case-dir".to_string())?;
        let dir = if dir.is_absolute() {
            dir.clone()
        } else {
            root_dir.join(dir)
        };

        let mut out: Vec<PathBuf> = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|e| format!("read_dir {dir:?}: {e}"))?;
        for ent in entries {
            let ent = ent.map_err(|e| format!("read_dir entry: {e}"))?;
            let path = ent.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            if ext == "yaml" || ext == "yml" {
                out.push(path);
            }
        }
        out.sort();
        return Ok(out);
    }

    Err("must provide --case <file> or --case-dir <dir> --all".to_string())
}

fn run_case(xin_bin: &Path, case: &Case) -> Result<(), String> {
    let run_id = generate_run_id();
    let mut ctx = Context {
        case_id: case.id.clone(),
        run_id,
        vars: HashMap::new(),
    };

    for (idx, step) in case.steps.iter().enumerate() {
        let step_name = step
            .name
            .clone()
            .unwrap_or_else(|| format!("step-{}", idx + 1));

        eprintln!("==> {}", step_name);
        for line in &step.say {
            eprintln!("  {line}");
        }

        run_step(xin_bin, &case.env, step, &mut ctx)
            .map_err(|e| format!("{step_name} failed: {e}"))?;
    }

    eprintln!(
        "{}",
        colorize(
            &format!("OK: case '{}' (runId={})", ctx.case_id, ctx.run_id),
            "92"
        )
    );
    Ok(())
}

fn read_case(path: &Path) -> Result<Case, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
    serde_yaml::from_str(&text).map_err(|e| format!("parse yaml: {e}"))
}

fn generate_run_id() -> String {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let pid = std::process::id();
    format!("{}-{}", ts, pid)
}

fn ensure_stalwart(stalwart_dir: &Path, fresh: bool) -> Result<(), String> {
    if fresh {
        // Stop + remove state.
        let down = stalwart_dir.join("scripts/down.sh");
        let _ = Command::new(&down).current_dir(stalwart_dir).status();
        let state_dir = stalwart_dir.join(".state");
        if state_dir.exists() {
            std::fs::remove_dir_all(&state_dir)
                .map_err(|e| format!("remove {state_dir:?}: {e}"))?;
        }
        let up = stalwart_dir.join("scripts/up.sh");
        run_cmd(Command::new(&up).current_dir(stalwart_dir))?;
        return Ok(());
    }

    // Non-fresh: start (or keep) the docker service. `docker compose up -d` is idempotent.
    let state_cfg = stalwart_dir.join(".state/opt-stalwart/etc/config.toml");
    if !state_cfg.exists() {
        let up = stalwart_dir.join("scripts/up.sh");
        run_cmd(Command::new(&up).current_dir(stalwart_dir))?;
        return Ok(());
    }

    run_cmd(
        Command::new("docker")
            .current_dir(stalwart_dir)
            .arg("compose")
            .arg("up")
            .arg("-d"),
    )?;

    Ok(())
}

fn run_step(
    xin_bin: &Path,
    case_env: &BTreeMap<String, String>,
    step: &Step,
    ctx: &mut Context,
) -> Result<(), String> {
    let retry = step.retry.as_ref();
    let attempts = retry.map(|r| r.attempts).unwrap_or(1);
    let sleep_ms = retry.and_then(|r| r.sleep_ms).unwrap_or(500);

    let mut last_err: Option<String> = None;

    for attempt in 1..=attempts {
        match run_step_once(xin_bin, case_env, step, ctx) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < attempts {
                    std::thread::sleep(Duration::from_millis(sleep_ms));
                    continue;
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| "unknown error".to_string()))
}

fn run_step_once(
    xin_bin: &Path,
    case_env: &BTreeMap<String, String>,
    step: &Step,
    ctx: &mut Context,
) -> Result<(), String> {
    let args: Vec<String> = step
        .xin
        .args
        .iter()
        .map(|s| substitute(s, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    // Merge env: case env first, then step env overrides.
    let mut merged_env: BTreeMap<String, String> = case_env.clone();
    for (k, v) in &step.env {
        merged_env.insert(k.clone(), v.clone());
    }

    let mut cmd = Command::new(xin_bin);
    for (k, v) in &merged_env {
        cmd.env(k, substitute(v, ctx)?);
    }

    let output = cmd
        .args(&args)
        .output()
        .map_err(|e| format!("spawn xin: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let value: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        format!(
            "stdout was not JSON envelope: {e}\nexit: {:?}\nstderr:\n{stderr}\nstdout:\n{stdout}",
            output.status.code()
        )
    })?;

    // Expect ok by default.
    if step.expect_ok {
        let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            return Err(format!(
                "expected ok=true but got ok=false\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ));
        }
    }

    // Explicit assertions.
    for a in &step.expect {
        let label = a.label.as_deref().unwrap_or("");
        let prefix = if label.is_empty() {
            format!("assert {}", a.path)
        } else {
            format!("assert {} ({})", a.path, label)
        };

        assert_one(&value, a, ctx).map_err(|e| format!("{prefix}: {e}\nstdout:\n{stdout}"))?;
    }

    // Save variables.
    for (var, ptr) in &step.save {
        let ptr = substitute(ptr, ctx)?;
        let v = value
            .pointer(&ptr)
            .ok_or_else(|| format!("save var {var}: missing pointer {ptr}\nstdout:\n{stdout}"))?;

        let s = v.as_str().ok_or_else(|| {
            format!("save var {var}: expected string at {ptr}, got {v}\nstdout:\n{stdout}")
        })?;

        ctx.vars.insert(var.to_string(), s.to_string());
    }

    Ok(())
}

fn assert_one(v: &serde_json::Value, a: &Assertion, ctx: &Context) -> Result<(), String> {
    let path = substitute(&a.path, ctx)?;
    let got = v
        .pointer(&path)
        .ok_or_else(|| format!("missing pointer {}", path))?;

    if a.exists.unwrap_or(false) {
        if got.is_null() {
            return Err("value is null".to_string());
        }
    }

    if let Some(eq) = &a.equals {
        // Allow variable substitution for string equals values.
        let eq = match eq {
            serde_yaml::Value::String(s) => serde_yaml::Value::String(substitute(s, ctx)?),
            other => other.clone(),
        };

        let expected: serde_json::Value =
            serde_json::to_value(eq).map_err(|e| format!("bad equals value: {e}"))?;
        if *got != expected {
            return Err(format!("expected {expected}, got {got}"));
        }
    }

    if let Some(substr) = &a.contains {
        let substr = substitute(substr, ctx)?;
        let s = got
            .as_str()
            .ok_or_else(|| format!("contains expects string, got {got}"))?;
        if !s.contains(&substr) {
            return Err(format!("expected to contain {substr:?}, got {s:?}"));
        }
    }

    Ok(())
}

fn substitute(input: &str, ctx: &Context) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    while let Some(start) = input[i..].find("${") {
        let abs_start = i + start;
        out.push_str(&input[i..abs_start]);

        let rest = &input[abs_start + 2..];
        let end = rest
            .find('}')
            .ok_or_else(|| format!("unterminated var in {input:?}"))?;

        let key = &rest[..end];
        let value = match key {
            "runId" => ctx.run_id.clone(),
            "caseId" => ctx.case_id.clone(),
            other => ctx
                .vars
                .get(other)
                .cloned()
                .ok_or_else(|| format!("unknown var {other:?} in {input:?}"))?,
        };

        out.push_str(&value);
        i = abs_start + 2 + end + 1;
    }

    out.push_str(&input[i..]);
    Ok(out)
}

fn run_cmd(cmd: &mut Command) -> Result<(), String> {
    let status = cmd
        .status()
        .map_err(|e| format!("failed to start command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with exit code {status}"))
    }
}

fn seed_stalwart(stalwart_dir: &Path, seed: &Seed) -> Result<(), String> {
    // Hard-coded local harness constants (kept in sync with tests/feature/stalwart).
    let base_url = "http://127.0.0.1:39090";
    let api = format!("{base_url}/api");
    let admin_user = "admin";
    let admin_pass = "xin-admin-pass";

    wait_ready(&api, admin_user, admin_pass)?;

    create_domain(&api, admin_user, admin_pass, &seed.domain)?;

    for u in &seed.users {
        let email = u
            .email
            .clone()
            .unwrap_or_else(|| format!("{}@{}", u.user, seed.domain));
        let roles = u.roles.clone().unwrap_or_else(|| vec!["user".to_string()]);
        create_user(
            &api, admin_user, admin_pass, &u.user, &u.pass, &email, &roles,
        )?;
    }

    // SMTP inject fixtures (optional)
    for inj in &seed.smtp_inject {
        smtp_inject(stalwart_dir, inj)?;
    }

    Ok(())
}

fn wait_ready(api: &str, user: &str, pass: &str) -> Result<(), String> {
    eprintln!("Waiting for management API at {api} ...");
    for _ in 0..60 {
        let status = Command::new("curl")
            .arg("-s")
            .arg("-o")
            .arg("/dev/null")
            .arg("--max-time")
            .arg("2")
            .arg("--connect-timeout")
            .arg("2")
            .arg("-u")
            .arg(format!("{user}:{pass}"))
            .arg("-H")
            .arg("Accept: application/json")
            .arg(format!("{api}/principal?limit=1"))
            .status();

        if matches!(status, Ok(s) if s.success()) {
            return Ok(());
        }

        std::thread::sleep(Duration::from_millis(500));
    }
    Err("management API did not become ready".to_string())
}

fn create_domain(api: &str, user: &str, pass: &str, domain: &str) -> Result<(), String> {
    eprintln!("Creating domain principal: {domain}");
    let v = curl_post_json_value(
        &format!("{api}/principal"),
        user,
        pass,
        &serde_json::json!({"type":"domain","name":domain}).to_string(),
    )?;

    accept_create_ok_or_exists(&v, "domain", domain)
}

fn create_user(
    api: &str,
    user: &str,
    pass: &str,
    principal: &str,
    principal_pass: &str,
    email: &str,
    roles: &[String],
) -> Result<(), String> {
    eprintln!("Creating user principal: {email}");
    let v = curl_post_json_value(
        &format!("{api}/principal"),
        user,
        pass,
        &serde_json::json!({
            "type":"individual",
            "name": principal,
            "emails": [email],
            "secrets": [principal_pass],
            "roles": roles,
        })
        .to_string(),
    )?;

    accept_create_ok_or_exists(&v, "user", principal)
}

fn smtp_inject(stalwart_dir: &Path, inj: &SmtpInject) -> Result<(), String> {
    let script = stalwart_dir.join("scripts/smtp_inject.py");
    let eml_path = if PathBuf::from(&inj.eml_file).is_absolute() {
        PathBuf::from(&inj.eml_file)
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&inj.eml_file)
    };

    let mut cmd = Command::new("python3");
    cmd.current_dir(stalwart_dir)
        .arg(script)
        .arg("--auth-user")
        .arg(&inj.auth_user)
        .arg("--auth-pass")
        .arg(&inj.auth_pass)
        .arg("--mail-from")
        .arg(&inj.mail_from);

    for rcpt in &inj.rcpt_to {
        cmd.arg("--rcpt-to").arg(rcpt);
    }

    cmd.arg("--eml").arg(eml_path);

    run_cmd(&mut cmd)
}

fn curl_post_json_value(
    url: &str,
    user: &str,
    pass: &str,
    body: &str,
) -> Result<serde_json::Value, String> {
    let out = Command::new("curl")
        .arg("-sS")
        .arg("-u")
        .arg(format!("{user}:{pass}"))
        .arg("-H")
        .arg("Accept: application/json")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-d")
        .arg(body)
        .output()
        .map_err(|e| format!("failed to start curl: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("seed api returned non-json: {e}\nstdout:\n{stdout}"))?;

    Ok(v)
}

fn accept_create_ok_or_exists(v: &serde_json::Value, what: &str, name: &str) -> Result<(), String> {
    if v.get("data").is_some() {
        return Ok(());
    }

    if v.get("error").and_then(|e| e.as_str()) == Some("fieldAlreadyExists") {
        return Ok(());
    }

    Err(format!("failed to create {what} {name}: {v}"))
}

fn fatal(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}
