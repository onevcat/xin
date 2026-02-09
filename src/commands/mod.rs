use crate::cli::*;
use crate::error::XinErrorOut;
use crate::output::Envelope;

mod labels;
mod organize;
mod read;
mod send;

pub async fn dispatch(cli: &Cli) -> Envelope<serde_json::Value> {
    let account = cli.account.clone();

    match &cli.command {
        Command::Search(args) => read::search("search", account.clone(), args).await,
        Command::Messages {
            command: MessagesCommand::Search(args),
        } => read::messages_search(account.clone(), args).await,

        // Phase 2: only READ first.
        Command::Get(args) => read::get(args).await,
        Command::Thread {
            command: ThreadCommand::Get(args),
        } => read::thread_get(args).await,
        Command::Thread {
            command: ThreadCommand::Attachments(args),
        } => read::thread_attachments(args).await,
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
        Command::Attachment(args) => read::attachment_download(args).await,
        Command::Archive(args) => organize::archive(account.clone(), args, cli.dry_run).await,
        Command::Read(args) => organize::read(account.clone(), args, cli.dry_run).await,
        Command::Unread(args) => organize::unread(account.clone(), args, cli.dry_run).await,
        Command::Trash(args) => organize::trash(account.clone(), args, cli.dry_run).await,
        Command::Batch {
            command: BatchCommand::Modify(args),
        } => organize::batch_modify(account.clone(), args, cli.dry_run).await,

        Command::Labels { command: sub } => match sub {
            LabelsCommand::List(args) => labels::list("labels.list", account.clone(), args).await,
            LabelsCommand::Get(args) => labels::get("labels.get", account.clone(), args).await,
            LabelsCommand::Create(args) => labels::create("labels.create", account.clone(), args).await,
            LabelsCommand::Rename(args) => labels::rename("labels.rename", account.clone(), args).await,
            LabelsCommand::Delete(args) => labels::delete("labels.delete", account.clone(), args).await,
            LabelsCommand::Modify(args) => labels::modify("labels.modify", account.clone(), args).await,
        },
        Command::Mailboxes { command: sub } => match sub {
            LabelsCommand::List(args) => labels::list("mailboxes.list", account.clone(), args).await,
            LabelsCommand::Get(args) => labels::get("mailboxes.get", account.clone(), args).await,
            LabelsCommand::Create(args) => labels::create("mailboxes.create", account.clone(), args).await,
            LabelsCommand::Rename(args) => labels::rename("mailboxes.rename", account.clone(), args).await,
            LabelsCommand::Delete(args) => labels::delete("mailboxes.delete", account.clone(), args).await,
            LabelsCommand::Modify(args) => labels::modify("mailboxes.modify", account.clone(), args).await,
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
            DraftsCommand::Delete(args) => send::drafts_delete(account.clone(), args).await,
            DraftsCommand::Send(args) => send::drafts_send(account.clone(), args).await,
            DraftsCommand::Update(args) => send::drafts_update(account.clone(), args).await,
        },

        _ => {
            let (command, _details) = command_name(&cli.command);
            Envelope::err(
                command,
                account,
                XinErrorOut::not_implemented("command not implemented yet"),
            )
        }
    }
}

fn command_name(cmd: &Command) -> (String, Option<String>) {
    match cmd {
        Command::Search(_) => ("search".to_string(), None),
        Command::Messages {
            command: MessagesCommand::Search(_),
        } => ("messages.search".to_string(), None),
        Command::Get(_) => ("get".to_string(), None),
        Command::Thread { command: sub } => match sub {
            ThreadCommand::Get(_) => ("thread.get".to_string(), None),
            ThreadCommand::Attachments(_) => ("thread.attachments".to_string(), None),
            ThreadCommand::Modify(_) => ("thread.modify".to_string(), None),
            ThreadCommand::Archive(_) => ("thread.archive".to_string(), None),
            ThreadCommand::Read(_) => ("thread.read".to_string(), None),
            ThreadCommand::Unread(_) => ("thread.unread".to_string(), None),
            ThreadCommand::Trash(_) => ("thread.trash".to_string(), None),
        },
        Command::Attachment(_) => ("attachment".to_string(), None),
        Command::Url(_) => ("url".to_string(), None),
        Command::Archive(_) => ("archive".to_string(), None),
        Command::Read(_) => ("read".to_string(), None),
        Command::Unread(_) => ("unread".to_string(), None),
        Command::Trash(_) => ("trash".to_string(), None),
        Command::Batch { command: sub } => match sub {
            BatchCommand::Modify(_) => ("batch.modify".to_string(), None),
            BatchCommand::Delete(_) => ("batch.delete".to_string(), None),
        },
        Command::Labels { command: sub } => match sub {
            LabelsCommand::List(_) => ("labels.list".to_string(), None),
            LabelsCommand::Get(_) => ("labels.get".to_string(), None),
            LabelsCommand::Create(_) => ("labels.create".to_string(), None),
            LabelsCommand::Rename(_) => ("labels.rename".to_string(), None),
            LabelsCommand::Delete(_) => ("labels.delete".to_string(), None),
            LabelsCommand::Modify(_) => ("labels.modify".to_string(), None),
        },
        Command::Mailboxes { command: sub } => match sub {
            LabelsCommand::List(_) => ("mailboxes.list".to_string(), None),
            LabelsCommand::Get(_) => ("mailboxes.get".to_string(), None),
            LabelsCommand::Create(_) => ("mailboxes.create".to_string(), None),
            LabelsCommand::Rename(_) => ("mailboxes.rename".to_string(), None),
            LabelsCommand::Delete(_) => ("mailboxes.delete".to_string(), None),
            LabelsCommand::Modify(_) => ("mailboxes.modify".to_string(), None),
        },
        Command::Identities { command: sub } => match sub {
            IdentitiesCommand::List => ("identities.list".to_string(), None),
            IdentitiesCommand::Get(_) => ("identities.get".to_string(), None),
        },
        Command::Send(_) => ("send".to_string(), None),
        Command::Drafts { command: sub } => match sub {
            DraftsCommand::List(_) => ("drafts.list".to_string(), None),
            DraftsCommand::Get(_) => ("drafts.get".to_string(), None),
            DraftsCommand::Create(_) => ("drafts.create".to_string(), None),
            DraftsCommand::Update(_) => ("drafts.update".to_string(), None),
            DraftsCommand::Delete(_) => ("drafts.delete".to_string(), None),
            DraftsCommand::Send(_) => ("drafts.send".to_string(), None),
        },
        Command::History(_) => ("history".to_string(), None),
        Command::Watch(_) => ("watch".to_string(), None),
    }
}
