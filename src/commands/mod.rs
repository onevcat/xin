use crate::cli::*;
use crate::output::Envelope;

mod auth_cmd;
mod config_cmd;
mod history;
mod inbox;
mod labels;
mod organize;
mod read;
mod send;
mod url;
mod watch;

pub async fn dispatch(cli: &Cli) -> Envelope<serde_json::Value> {
    let account = cli.account.clone();

    match &cli.command {
        Command::Search(args) => read::search("search", account.clone(), args).await,
        Command::Messages {
            command: MessagesCommand::Search(args),
        } => read::messages_search(account.clone(), args).await,

        // Phase 2: only READ first.
        Command::Get(args) => read::get(account.clone(), args).await,
        Command::Thread {
            command: ThreadCommand::Get(args),
        } => read::thread_get(account.clone(), args).await,
        Command::Thread {
            command: ThreadCommand::Attachments(args),
        } => read::thread_attachments(account.clone(), args).await,
        Command::Thread {
            command: ThreadCommand::Modify(args),
        } => organize::thread_modify(account.clone(), args, cli.dry_run).await,
        Command::Thread {
            command: ThreadCommand::Archive(args),
        } => organize::thread_archive(account.clone(), args, cli.dry_run).await,
        Command::Thread {
            command: ThreadCommand::Read(args),
        } => organize::thread_read(account.clone(), args, cli.dry_run).await,
        Command::Thread {
            command: ThreadCommand::Unread(args),
        } => organize::thread_unread(account.clone(), args, cli.dry_run).await,
        Command::Thread {
            command: ThreadCommand::Trash(args),
        } => organize::thread_trash(account.clone(), args, cli.dry_run).await,
        Command::Thread {
            command: ThreadCommand::Delete(args),
        } => organize::thread_delete(account.clone(), args, cli.dry_run, cli.force).await,
        Command::Attachment(args) => read::attachment_download(account.clone(), args).await,
        Command::Url(args) => url::url(account.clone(), args).await,
        Command::Archive(args) => organize::archive(account.clone(), args, cli.dry_run).await,
        Command::Read(args) => organize::read(account.clone(), args, cli.dry_run).await,
        Command::Unread(args) => organize::unread(account.clone(), args, cli.dry_run).await,
        Command::Trash(args) => organize::trash(account.clone(), args, cli.dry_run).await,
        Command::Batch {
            command: BatchCommand::Modify(args),
        } => organize::batch_modify(account.clone(), args, cli.dry_run).await,
        Command::Batch {
            command: BatchCommand::Delete(args),
        } => organize::batch_delete(account.clone(), args, cli.dry_run, cli.force).await,

        Command::Inbox { command: sub } => match sub {
            InboxCommand::Next(args) => inbox::next(account.clone(), args).await,
            InboxCommand::Do(args) => inbox::do_action(account.clone(), args, cli.dry_run).await,
        },

        Command::Labels { command: sub } => match sub {
            LabelsCommand::List(args) => labels::list("labels.list", account.clone(), args).await,
            LabelsCommand::Get(args) => labels::get("labels.get", account.clone(), args).await,
            LabelsCommand::Create(args) => {
                labels::create("labels.create", account.clone(), args).await
            }
            LabelsCommand::Rename(args) => {
                labels::rename("labels.rename", account.clone(), args).await
            }
            LabelsCommand::Delete(args) => {
                labels::delete("labels.delete", account.clone(), args).await
            }
            LabelsCommand::Modify(args) => {
                labels::modify("labels.modify", account.clone(), args).await
            }
        },
        Command::Mailboxes { command: sub } => match sub {
            LabelsCommand::List(args) => {
                labels::list("mailboxes.list", account.clone(), args).await
            }
            LabelsCommand::Get(args) => labels::get("mailboxes.get", account.clone(), args).await,
            LabelsCommand::Create(args) => {
                labels::create("mailboxes.create", account.clone(), args).await
            }
            LabelsCommand::Rename(args) => {
                labels::rename("mailboxes.rename", account.clone(), args).await
            }
            LabelsCommand::Delete(args) => {
                labels::delete("mailboxes.delete", account.clone(), args).await
            }
            LabelsCommand::Modify(args) => {
                labels::modify("mailboxes.modify", account.clone(), args).await
            }
        },

        Command::Identities {
            command: IdentitiesCommand::List,
        } => send::identities_list(account.clone()).await,
        Command::Identities {
            command: IdentitiesCommand::Get(args),
        } => send::identities_get(account.clone(), args).await,

        Command::Send(args) => send::send(account.clone(), args).await,

        Command::Drafts { command: sub } => match sub {
            DraftsCommand::List(args) => send::drafts_list(account.clone(), args).await,
            DraftsCommand::Get(args) => send::drafts_get(account.clone(), args).await,
            DraftsCommand::Create(args) => send::drafts_create(account.clone(), args).await,
            DraftsCommand::Update(args) => send::drafts_update(account.clone(), args).await,
            DraftsCommand::Rewrite(args) => {
                send::drafts_rewrite(account.clone(), args, cli.force).await
            }
            DraftsCommand::Delete(args) => send::drafts_delete(account.clone(), args).await,
            DraftsCommand::Destroy(args) => {
                send::drafts_destroy(account.clone(), args, cli.force).await
            }
            DraftsCommand::Send(args) => send::drafts_send(account.clone(), args).await,
        },

        Command::History(args) => history::history(account.clone(), args).await,
        Command::Watch(args) => watch::watch(account.clone(), args).await,

        Command::Config { command: sub } => match sub {
            ConfigCommand::Init => config_cmd::init().await,
            ConfigCommand::List => config_cmd::list().await,
            ConfigCommand::SetDefault(args) => config_cmd::set_default(args).await,
            ConfigCommand::Show(args) => config_cmd::show(account.as_deref(), args).await,
        },

        Command::Auth { command: sub } => match sub {
            AuthCommand::SetToken(args) => auth_cmd::set_token(account.as_deref(), args).await,
        },

    }
}
