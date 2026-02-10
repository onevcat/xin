use serde_json::{Value, json};

use crate::cli::{InboxAction, InboxDoArgs, InboxNextArgs, SearchArgs, SortKey};
use crate::output::Envelope;

use super::{organize, read};

pub async fn next(account: Option<String>, args: &InboxNextArgs) -> Envelope<Value> {
    let mut q = String::from("in:inbox");
    if !args.all {
        q.push_str(" seen:false");
    }
    if let Some(extra) = &args.query {
        let extra = extra.trim();
        if !extra.is_empty() {
            q.push(' ');
            q.push_str(extra);
        }
    }

    let search_args = SearchArgs {
        query: Some(q.clone()),
        max: Some(args.max.unwrap_or(1)),
        page: args.page.clone(),
        oldest: args.oldest,
        filter_json: None,
        collapse_threads: Some(false),
        sort: SortKey::ReceivedAt,
    };

    let mut env = read::search("inbox.next", account, &search_args).await;

    if !env.ok {
        return env;
    }

    // Add a convenience `item` field, keeping `items` intact.
    if let Some(data) = env.data.as_mut() {
        if let Some(obj) = data.as_object_mut() {
            let first = obj
                .get("items")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .cloned()
                .unwrap_or(Value::Null);
            obj.insert("item".to_string(), first);
            obj.insert(
                "query".to_string(),
                json!({
                    "compiled": q,
                    "collapseThreads": false,
                    "oldest": args.oldest,
                    "all": args.all
                }),
            );
        }
    }

    env
}

pub async fn do_action(
    account: Option<String>,
    args: &InboxDoArgs,
    dry_run: bool,
) -> Envelope<Value> {
    // v0: keep it simple; map to existing organize sugar.
    let command_name = "inbox.do";

    match args.action {
        InboxAction::Archive => {
            let a = crate::cli::ArchiveArgs {
                email_ids: vec![args.email_id.clone()],
                whole_thread: args.whole_thread,
            };
            let mut env = organize::archive(account, &a, dry_run).await;
            env.command = command_name.to_string();
            env
        }
        InboxAction::Trash => {
            let a = crate::cli::TrashArgs {
                email_ids: vec![args.email_id.clone()],
                whole_thread: args.whole_thread,
            };
            let mut env = organize::trash(account, &a, dry_run).await;
            env.command = command_name.to_string();
            env
        }
        InboxAction::Read => {
            let a = crate::cli::ReadArgs {
                email_ids: vec![args.email_id.clone()],
                whole_thread: args.whole_thread,
            };
            let mut env = organize::read(account, &a, dry_run).await;
            env.command = command_name.to_string();
            env
        }
        InboxAction::Unread => {
            let a = crate::cli::UnreadArgs {
                email_ids: vec![args.email_id.clone()],
                whole_thread: args.whole_thread,
            };
            let mut env = organize::unread(account, &a, dry_run).await;
            env.command = command_name.to_string();
            env
        }
    }
}
