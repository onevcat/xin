use serde_json::{Value, json};

use crate::backend::{Backend, ModifyPlan};
use crate::cli::{
    ArchiveArgs, BatchDeleteArgs, BatchModifyArgs, ReadArgs, ThreadArchiveArgs, ThreadDeleteArgs,
    ThreadModifyArgs, ThreadReadArgs, ThreadTrashArgs, ThreadUnreadArgs, TrashArgs, UnreadArgs,
};
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

#[derive(Debug, Clone, Default)]
struct ChangeSummary {
    added_mailboxes: Vec<String>,
    removed_mailboxes: Vec<String>,
    added_keywords: Vec<String>,
    removed_keywords: Vec<String>,
}

impl ChangeSummary {
    fn to_json(&self) -> Value {
        json!({
            "mailboxIds": {"added": self.added_mailboxes, "removed": self.removed_mailboxes},
            "keywords": {"added": self.added_keywords, "removed": self.removed_keywords}
        })
    }
}

// ModifyPlan is defined in backend.rs

fn resolve_mailbox_id(s: &str, mailboxes: &[jmap_client::mailbox::Mailbox]) -> Option<String> {
    let needle = s.trim();
    if needle.is_empty() {
        return None;
    }

    // 0) direct id match
    for m in mailboxes {
        if let Some(id) = m.id() {
            if id == needle {
                return Some(id.to_string());
            }
        }
    }

    // 1) role match
    let needle_lower = needle.to_lowercase();
    let role = match needle_lower.as_str() {
        "spam" => "junk",
        "bin" => "trash",
        other => other,
    };

    let role_match = mailboxes.iter().find(|m| {
        use jmap_client::mailbox::Role;
        match (role, m.role()) {
            ("inbox", Role::Inbox)
            | ("trash", Role::Trash)
            | ("junk", Role::Junk)
            | ("sent", Role::Sent)
            | ("drafts", Role::Drafts)
            | ("archive", Role::Archive)
            | ("important", Role::Important) => true,
            (other, Role::Other(s)) => other == s.to_lowercase(),
            _ => false,
        }
    });

    if let Some(m) = role_match {
        return m.id().map(|id| id.to_string());
    }

    // 2) exact name match
    if let Some(m) = mailboxes.iter().find(|m| m.name() == Some(needle)) {
        return m.id().map(|id| id.to_string());
    }

    // 3) case-insensitive name match
    let lower = needle.to_lowercase();
    if let Some(m) = mailboxes
        .iter()
        .find(|m| m.name().map(|n| n.to_lowercase()) == Some(lower.clone()))
    {
        return m.id().map(|id| id.to_string());
    }

    None
}

fn parse_modify_args_auto(
    add: &[String],
    remove: &[String],
    add_mailbox: &[String],
    remove_mailbox: &[String],
    add_keyword: &[String],
    remove_keyword: &[String],
    mailboxes: &[jmap_client::mailbox::Mailbox],
) -> Result<(ModifyPlan, ChangeSummary), XinErrorOut> {
    let mut plan = ModifyPlan::default();
    let mut summary = ChangeSummary::default();

    // Explicit mailbox/keyword flags.
    for m in add_mailbox {
        let id = resolve_mailbox_id(m, mailboxes)
            .ok_or_else(|| XinErrorOut::usage(format!("unknown mailbox: {m}")))?;
        plan.add_mailboxes.push(id.clone());
        summary.added_mailboxes.push(id);
    }

    for m in remove_mailbox {
        let id = resolve_mailbox_id(m, mailboxes)
            .ok_or_else(|| XinErrorOut::usage(format!("unknown mailbox: {m}")))?;
        plan.remove_mailboxes.push(id.clone());
        summary.removed_mailboxes.push(id);
    }

    for k in add_keyword {
        plan.add_keywords.push(k.clone());
        summary.added_keywords.push(k.clone());
    }

    for k in remove_keyword {
        plan.remove_keywords.push(k.clone());
        summary.removed_keywords.push(k.clone());
    }

    // Auto route: mailbox if resolvable, otherwise keyword.
    for t in add {
        if let Some(id) = resolve_mailbox_id(t, mailboxes) {
            plan.add_mailboxes.push(id.clone());
            summary.added_mailboxes.push(id);
        } else {
            plan.add_keywords.push(t.clone());
            summary.added_keywords.push(t.clone());
        }
    }

    for t in remove {
        if let Some(id) = resolve_mailbox_id(t, mailboxes) {
            plan.remove_mailboxes.push(id.clone());
            summary.removed_mailboxes.push(id);
        } else {
            plan.remove_keywords.push(t.clone());
            summary.removed_keywords.push(t.clone());
        }
    }

    Ok((plan, summary))
}

async fn apply_plan_to_emails(
    backend: &Backend,
    email_ids: &[String],
    plan: &ModifyPlan,
    dry_run: bool,
) -> Result<(), XinErrorOut> {
    if dry_run {
        return Ok(());
    }

    backend.modify_emails(email_ids, plan).await
}

pub async fn batch_modify(
    account: Option<String>,
    args: &BatchModifyArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "batch.modify";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let (plan, summary) = match parse_modify_args_auto(
        &args.add,
        &args.remove,
        &args.add_mailbox,
        &args.remove_mailbox,
        &args.add_keyword,
        &args.remove_keyword,
        &mailboxes,
    ) {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if plan.add_mailboxes.is_empty()
        && plan.remove_mailboxes.is_empty()
        && plan.add_keywords.is_empty()
        && plan.remove_keywords.is_empty()
    {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("no changes specified".to_string()),
        );
    }

    if let Err(e) = apply_plan_to_emails(&backend, &args.email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn batch_delete(
    account: Option<String>,
    args: &BatchDeleteArgs,
    dry_run: bool,
    force: bool,
) -> Envelope<Value> {
    let command_name = "batch.delete";

    if !force {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("batch delete is destructive; pass --force".to_string()),
        );
    }

    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if !dry_run {
        if let Err(e) = backend.destroy_emails(&args.email_ids).await {
            return Envelope::err(command_name, account, e);
        }
    }

    let deleted = args.email_ids.clone();

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "deleted": deleted,
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn thread_modify(
    account: Option<String>,
    args: &ThreadModifyArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "thread.modify";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let (plan, summary) = match parse_modify_args_auto(
        &args.add,
        &args.remove,
        &args.add_mailbox,
        &args.remove_mailbox,
        &args.add_keyword,
        &args.remove_keyword,
        &mailboxes,
    ) {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if plan.add_mailboxes.is_empty()
        && plan.remove_mailboxes.is_empty()
        && plan.add_keywords.is_empty()
        && plan.remove_keywords.is_empty()
    {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("no changes specified".to_string()),
        );
    }

    let email_ids = match backend.thread_email_ids(&args.thread_id).await {
        Ok(Some(ids)) => ids,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("thread not found".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if let Err(e) = apply_plan_to_emails(&backend, &email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"threadId": args.thread_id, "emailIds": email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

async fn thread_sugar(
    backend: &Backend,
    command_name: &str,
    account: Option<String>,
    thread_id: &str,
    plan: ModifyPlan,
    summary: ChangeSummary,
    dry_run: bool,
) -> Envelope<Value> {
    let email_ids = match backend.thread_email_ids(thread_id).await {
        Ok(Some(ids)) => ids,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("thread not found".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if let Err(e) = apply_plan_to_emails(backend, &email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"threadId": thread_id, "emailIds": email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn thread_archive(
    account: Option<String>,
    args: &ThreadArchiveArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "thread.archive";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let inbox_id = match resolve_mailbox_id("inbox", &mailboxes) {
        Some(id) => id,
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("inbox mailbox not found".to_string()),
            );
        }
    };

    let archive_id = resolve_mailbox_id("archive", &mailboxes);

    let mut plan = ModifyPlan::default();
    plan.remove_mailboxes.push(inbox_id.clone());
    let mut summary = ChangeSummary::default();
    summary.removed_mailboxes.push(inbox_id);

    if let Some(aid) = archive_id {
        plan.add_mailboxes.push(aid.clone());
        summary.added_mailboxes.push(aid);
    }

    thread_sugar(
        &backend,
        command_name,
        account,
        &args.thread_id,
        plan,
        summary,
        dry_run,
    )
    .await
}

pub async fn thread_read(
    account: Option<String>,
    args: &ThreadReadArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "thread.read";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mut plan = ModifyPlan::default();
    plan.add_keywords.push("$seen".to_string());
    let mut summary = ChangeSummary::default();
    summary.added_keywords.push("$seen".to_string());

    thread_sugar(
        &backend,
        command_name,
        account,
        &args.thread_id,
        plan,
        summary,
        dry_run,
    )
    .await
}

pub async fn thread_unread(
    account: Option<String>,
    args: &ThreadUnreadArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "thread.unread";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mut plan = ModifyPlan::default();
    plan.remove_keywords.push("$seen".to_string());
    let mut summary = ChangeSummary::default();
    summary.removed_keywords.push("$seen".to_string());

    thread_sugar(
        &backend,
        command_name,
        account,
        &args.thread_id,
        plan,
        summary,
        dry_run,
    )
    .await
}

pub async fn thread_trash(
    account: Option<String>,
    args: &ThreadTrashArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "thread.trash";

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };
    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };
    let trash_id = match resolve_mailbox_id("trash", &mailboxes) {
        Some(id) => id,
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("trash mailbox not found".to_string()),
            );
        }
    };
    let mut plan = ModifyPlan::default();
    plan.replace_mailboxes = Some(vec![trash_id.clone()]);

    let mut summary = ChangeSummary::default();
    summary.added_mailboxes.push(trash_id);

    thread_sugar(
        &backend,
        command_name,
        account,
        &args.thread_id,
        plan,
        summary,
        dry_run,
    )
    .await
}

pub async fn thread_delete(
    account: Option<String>,
    args: &ThreadDeleteArgs,
    dry_run: bool,
    force: bool,
) -> Envelope<Value> {
    let command_name = "thread.delete";

    if !force {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("thread delete is destructive; pass --force".to_string()),
        );
    }

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let email_ids = match backend.thread_email_ids(&args.thread_id).await {
        Ok(Some(ids)) => ids,
        Ok(None) => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("thread not found".to_string()),
            );
        }
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let deleted = email_ids.clone();

    if email_ids.is_empty() {
        return Envelope::ok(
            command_name,
            account,
            json!({
                "appliedTo": {"threadId": args.thread_id, "emailIds": email_ids},
                "deleted": deleted,
                "dryRun": dry_run
            }),
            Meta::default(),
        );
    }

    if !dry_run {
        if let Err(e) = backend.destroy_emails(&email_ids).await {
            return Envelope::err(command_name, account, e);
        }
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"threadId": args.thread_id, "emailIds": email_ids},
            "deleted": deleted,
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn archive(
    account: Option<String>,
    args: &ArchiveArgs,
    dry_run: bool,
) -> Envelope<Value> {
    let command_name = "archive";

    if args.whole_thread && args.email_ids.len() != 1 {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("--whole-thread requires exactly one emailId".to_string()),
        );
    }

    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let inbox_id = match resolve_mailbox_id("inbox", &mailboxes) {
        Some(id) => id,
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("inbox mailbox not found".to_string()),
            );
        }
    };
    let archive_id = resolve_mailbox_id("archive", &mailboxes);

    let mut plan = ModifyPlan::default();
    plan.remove_mailboxes.push(inbox_id.clone());
    let mut summary = ChangeSummary::default();
    summary.removed_mailboxes.push(inbox_id);

    if let Some(aid) = archive_id {
        plan.add_mailboxes.push(aid.clone());
        summary.added_mailboxes.push(aid);
    }

    if args.whole_thread {
        let thread_id = match backend
            .get_email(
                &args.email_ids[0],
                Some(vec![jmap_client::email::Property::ThreadId]),
            )
            .await
        {
            Ok(Some(e)) => e
                .thread_id()
                .map(|s| s.to_string())
                .ok_or_else(|| XinErrorOut::config("email missing threadId".to_string())),
            Ok(None) => Err(XinErrorOut::usage("email not found".to_string())),
            Err(e) => Err(e),
        };

        let thread_id = match thread_id {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        };

        return thread_sugar(
            &backend,
            command_name,
            account,
            &thread_id,
            plan,
            summary,
            dry_run,
        )
        .await;
    }

    if let Err(e) = apply_plan_to_emails(&backend, &args.email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn read(account: Option<String>, args: &ReadArgs, dry_run: bool) -> Envelope<Value> {
    let command_name = "read";

    if args.whole_thread && args.email_ids.len() != 1 {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("--whole-thread requires exactly one emailId".to_string()),
        );
    }
    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let mut plan = ModifyPlan::default();
    plan.add_keywords.push("$seen".to_string());

    let mut summary = ChangeSummary::default();
    summary.added_keywords.push("$seen".to_string());

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.whole_thread {
        let thread_id = match backend
            .get_email(
                &args.email_ids[0],
                Some(vec![jmap_client::email::Property::ThreadId]),
            )
            .await
        {
            Ok(Some(e)) => e
                .thread_id()
                .map(|s| s.to_string())
                .ok_or_else(|| XinErrorOut::config("email missing threadId".to_string())),
            Ok(None) => Err(XinErrorOut::usage("email not found".to_string())),
            Err(e) => Err(e),
        };

        let thread_id = match thread_id {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        };

        return thread_sugar(
            &backend,
            command_name,
            account,
            &thread_id,
            plan,
            summary,
            dry_run,
        )
        .await;
    }

    if let Err(e) = apply_plan_to_emails(&backend, &args.email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn unread(account: Option<String>, args: &UnreadArgs, dry_run: bool) -> Envelope<Value> {
    let command_name = "unread";

    if args.whole_thread && args.email_ids.len() != 1 {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("--whole-thread requires exactly one emailId".to_string()),
        );
    }
    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let mut plan = ModifyPlan::default();
    plan.remove_keywords.push("$seen".to_string());

    let mut summary = ChangeSummary::default();
    summary.removed_keywords.push("$seen".to_string());

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    if args.whole_thread {
        let thread_id = match backend
            .get_email(
                &args.email_ids[0],
                Some(vec![jmap_client::email::Property::ThreadId]),
            )
            .await
        {
            Ok(Some(e)) => e
                .thread_id()
                .map(|s| s.to_string())
                .ok_or_else(|| XinErrorOut::config("email missing threadId".to_string())),
            Ok(None) => Err(XinErrorOut::usage("email not found".to_string())),
            Err(e) => Err(e),
        };

        let thread_id = match thread_id {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        };

        return thread_sugar(
            &backend,
            command_name,
            account,
            &thread_id,
            plan,
            summary,
            dry_run,
        )
        .await;
    }

    if let Err(e) = apply_plan_to_emails(&backend, &args.email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}

pub async fn trash(account: Option<String>, args: &TrashArgs, dry_run: bool) -> Envelope<Value> {
    let command_name = "trash";

    if args.whole_thread && args.email_ids.len() != 1 {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("--whole-thread requires exactly one emailId".to_string()),
        );
    }
    if args.email_ids.is_empty() {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("missing emailId".to_string()),
        );
    }

    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let trash_id = match resolve_mailbox_id("trash", &mailboxes) {
        Some(id) => id,
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::config("trash mailbox not found".to_string()),
            );
        }
    };

    let mut plan = ModifyPlan::default();
    plan.replace_mailboxes = Some(vec![trash_id.clone()]);

    let mut summary = ChangeSummary::default();
    summary.added_mailboxes.push(trash_id);

    if args.whole_thread {
        let thread_id = match backend
            .get_email(
                &args.email_ids[0],
                Some(vec![jmap_client::email::Property::ThreadId]),
            )
            .await
        {
            Ok(Some(e)) => e
                .thread_id()
                .map(|s| s.to_string())
                .ok_or_else(|| XinErrorOut::config("email missing threadId".to_string())),
            Ok(None) => Err(XinErrorOut::usage("email not found".to_string())),
            Err(e) => Err(e),
        };

        let thread_id = match thread_id {
            Ok(t) => t,
            Err(e) => return Envelope::err(command_name, account, e),
        };

        return thread_sugar(
            &backend,
            command_name,
            account,
            &thread_id,
            plan,
            summary,
            dry_run,
        )
        .await;
    }

    if let Err(e) = apply_plan_to_emails(&backend, &args.email_ids, &plan, dry_run).await {
        return Envelope::err(command_name, account, e);
    }

    Envelope::ok(
        command_name,
        account,
        json!({
            "appliedTo": {"emailIds": args.email_ids},
            "changes": summary.to_json(),
            "dryRun": dry_run
        }),
        Meta::default(),
    )
}
