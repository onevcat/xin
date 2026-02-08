use clap::Parser;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
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
    #[arg(long)]
    case: PathBuf,

    /// Directory containing the Stalwart docker setup (tests/feature/stalwart).
    #[arg(long, default_value = "tests/feature/stalwart")]
    stalwart_dir: PathBuf,

    /// Reset the docker server state before running (down + rm -rf .state + up + seed).
    #[arg(long)]
    fresh: bool,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,

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
    let case_path = if cli.case.is_absolute() {
        cli.case.clone()
    } else {
        root_dir.join(&cli.case)
    };

    let case =
        read_case(&case_path).unwrap_or_else(|e| fatal(&format!("failed to read case: {e}")));

    let fresh = cli.fresh || case.requires_fresh;

    let stalwart_dir = root_dir.join(&cli.stalwart_dir);
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

    // Ensure xin is built (debug) so we can execute it by path.
    run_cmd(
        Command::new("cargo")
            .current_dir(&root_dir)
            .arg("build")
            .arg("-q"),
    )
    .unwrap_or_else(|e| fatal(&format!("cargo build failed: {e}")));

    let xin_bin = root_dir.join("target/debug/xin");

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

        run_step(&xin_bin, &case.env, step, &mut ctx)
            .unwrap_or_else(|e| fatal(&format!("{step_name} failed: {e}")));
    }

    eprintln!("OK: case '{}' (runId={})", ctx.case_id, ctx.run_id);
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

    // Non-fresh: if the server is down, try starting it.
    let seed = stalwart_dir.join("scripts/seed.sh");
    let seed_status = Command::new(&seed)
        .current_dir(stalwart_dir)
        .status()
        .map_err(|e| format!("run seed.sh: {e}"))?;

    if seed_status.success() {
        return Ok(());
    }

    let up = stalwart_dir.join("scripts/up.sh");
    run_cmd(Command::new(&up).current_dir(stalwart_dir))?;

    Ok(())
}

fn run_step(
    xin_bin: &Path,
    env: &BTreeMap<String, String>,
    step: &Step,
    ctx: &mut Context,
) -> Result<(), String> {
    let retry = step.retry.as_ref();
    let attempts = retry.map(|r| r.attempts).unwrap_or(1);
    let sleep_ms = retry.and_then(|r| r.sleep_ms).unwrap_or(500);

    let mut last_err: Option<String> = None;

    for attempt in 1..=attempts {
        match run_step_once(xin_bin, env, step, ctx) {
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
    env: &BTreeMap<String, String>,
    step: &Step,
    ctx: &mut Context,
) -> Result<(), String> {
    let args: Vec<String> = step
        .xin
        .args
        .iter()
        .map(|s| substitute(s, ctx))
        .collect::<Result<Vec<_>, _>>()?;

    let mut cmd = Command::new(xin_bin);
    for (k, v) in env {
        cmd.env(k, substitute(v, ctx)?);
    }

    let output = cmd
        .args(&args)
        .output()
        .map_err(|e| format!("spawn xin: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("stdout was not JSON envelope: {e}\nstdout:\n{stdout}"))?;

    // Expect ok by default.
    if step.expect_ok {
        let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            return Err(format!("expected ok=true but got: {value}"));
        }
    }

    // Explicit assertions.
    for a in &step.expect {
        assert_one(&value, a, ctx).map_err(|e| format!("assert {}: {e}", a.path))?;
    }

    // Save variables.
    for (var, ptr) in &step.save {
        let ptr = substitute(ptr, ctx)?;
        let v = value
            .pointer(&ptr)
            .ok_or_else(|| format!("save var {var}: missing pointer {ptr}"))?;

        let s = v
            .as_str()
            .ok_or_else(|| format!("save var {var}: expected string at {ptr}, got {v}"))?;

        ctx.vars.insert(var.to_string(), s.to_string());
    }

    Ok(())
}

fn assert_one(v: &serde_json::Value, a: &Assertion, ctx: &Context) -> Result<(), String> {
    let got = v
        .pointer(&a.path)
        .ok_or_else(|| format!("missing pointer {}", a.path))?;

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

// (no cmd_output helper; keep warnings clean)

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

// (removed) curl_http_code: unused

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
