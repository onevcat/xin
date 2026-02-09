use serde_json::{Value, json};

use crate::backend::Backend;
use crate::cli::{
    LabelsCreateArgs, LabelsDeleteArgs, LabelsGetArgs, LabelsListArgs, LabelsModifyArgs,
    LabelsRenameArgs,
};
use crate::error::XinErrorOut;
use crate::output::{Envelope, Meta};

fn role_to_string(role: &jmap_client::mailbox::Role) -> Option<String> {
    use jmap_client::mailbox::Role;
    match role {
        Role::Inbox => Some("inbox".to_string()),
        Role::Trash => Some("trash".to_string()),
        Role::Junk => Some("junk".to_string()),
        Role::Sent => Some("sent".to_string()),
        Role::Drafts => Some("drafts".to_string()),
        Role::Archive => Some("archive".to_string()),
        Role::Important => Some("important".to_string()),
        Role::Other(s) => Some(s.to_string()),
        Role::None => None,
    }
}

fn mailbox_to_json(m: &jmap_client::mailbox::Mailbox) -> Value {
    json!({
        "id": m.id(),
        "name": m.name(),
        "role": role_to_string(&m.role()),
        "parentId": m.parent_id(),
        "sortOrder": m.sort_order(),
        "totalEmails": m.total_emails(),
        "unreadEmails": m.unread_emails(),
        "totalThreads": m.total_threads(),
        "unreadThreads": m.unread_threads(),
        "isSubscribed": m.is_subscribed(),
    })
}

fn parse_role(role: &str) -> jmap_client::mailbox::Role {
    use jmap_client::mailbox::Role;
    match role.to_lowercase().as_str() {
        "inbox" => Role::Inbox,
        "trash" => Role::Trash,
        "junk" | "spam" => Role::Junk,
        "sent" => Role::Sent,
        "drafts" => Role::Drafts,
        "archive" => Role::Archive,
        "important" => Role::Important,
        other => Role::Other(other.to_string()),
    }
}

fn resolve_mailbox_id(
    s: &str,
    mailboxes: &[jmap_client::mailbox::Mailbox],
) -> Option<String> {
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

pub async fn list(
    command_name: &str,
    account: Option<String>,
    _args: &LabelsListArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    Envelope::ok(
        command_name,
        account,
        json!({
            "mailboxes": mailboxes.iter().map(mailbox_to_json).collect::<Vec<_>>()
        }),
        Meta::default(),
    )
}

pub async fn get(
    command_name: &str,
    account: Option<String>,
    args: &LabelsGetArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let mailboxes = match backend.list_mailboxes().await {
        Ok(m) => m,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let id = match resolve_mailbox_id(&args.mailbox, &mailboxes) {
        Some(id) => id,
        None => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage(format!("unknown mailbox: {}", args.mailbox)),
            );
        }
    };

    let mailbox = mailboxes
        .into_iter()
        .find(|m| m.id() == Some(id.as_str()));

    match mailbox {
        Some(m) => Envelope::ok(
            command_name,
            account,
            json!({"mailbox": mailbox_to_json(&m)}),
            Meta::default(),
        ),
        None => Envelope::err(
            command_name,
            account,
            XinErrorOut::usage(format!("mailbox not found: {}", id)),
        ),
    }
}

pub async fn create(
    command_name: &str,
    account: Option<String>,
    args: &LabelsCreateArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let role = args.role.as_deref().map(parse_role);

    let created = match backend
        .create_mailbox(
            args.name.clone(),
            args.parent.clone(),
            role,
            args.subscribe,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    Envelope::ok(
        command_name,
        account,
        json!({
            "created": [created],
            "updated": [],
            "destroyed": []
        }),
        Meta::default(),
    )
}

pub async fn rename(
    command_name: &str,
    account: Option<String>,
    args: &LabelsRenameArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    match backend.rename_mailbox(&args.mailbox_id, &args.name).await {
        Ok(()) => Envelope::ok(
            command_name,
            account,
            json!({
                "created": [],
                "updated": [{"id": args.mailbox_id}],
                "destroyed": []
            }),
            Meta::default(),
        ),
        Err(e) => Envelope::err(command_name, account, e),
    }
}

pub async fn modify(
    command_name: &str,
    account: Option<String>,
    args: &LabelsModifyArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    let sort_order = match args.sort_order {
        Some(v) if v < 0 => {
            return Envelope::err(
                command_name,
                account,
                XinErrorOut::usage("sort-order must be non-negative".to_string()),
            );
        }
        Some(v) => Some(v as u32),
        None => None,
    };

    if args.name.is_none()
        && args.parent.is_none()
        && sort_order.is_none()
        && args.subscribe.is_none()
    {
        return Envelope::err(
            command_name,
            account,
            XinErrorOut::usage("no changes specified".to_string()),
        );
    }

    match backend
        .modify_mailbox(
            &args.mailbox_id,
            args.name.clone(),
            args.parent.clone(),
            sort_order,
            args.subscribe,
        )
        .await
    {
        Ok(()) => Envelope::ok(
            command_name,
            account,
            json!({
                "created": [],
                "updated": [{"id": args.mailbox_id}],
                "destroyed": []
            }),
            Meta::default(),
        ),
        Err(e) => Envelope::err(command_name, account, e),
    }
}

pub async fn delete(
    command_name: &str,
    account: Option<String>,
    args: &LabelsDeleteArgs,
) -> Envelope<Value> {
    let backend = match Backend::connect().await {
        Ok(b) => b,
        Err(e) => return Envelope::err(command_name, account, e),
    };

    match backend
        .destroy_mailbox(&args.mailbox_id, args.remove_emails)
        .await
    {
        Ok(()) => Envelope::ok(
            command_name,
            account,
            json!({
                "created": [],
                "updated": [],
                "destroyed": [args.mailbox_id]
            }),
            Meta::default(),
        ),
        Err(e) => Envelope::err(command_name, account, e),
    }
}
