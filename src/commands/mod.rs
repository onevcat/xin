use crate::cli::*;
use crate::error::XinErrorOut;
use crate::output::Envelope;

pub fn dispatch(cli: &Cli) -> Envelope<serde_json::Value> {
    // Phase 1: command surface only; everything returns NotImplemented.
    // Keep the command string stable-ish for agent parsing.
    let account = cli.account.clone();

    let (command, _details) = command_name(&cli.command);

    Envelope::err(
        command,
        account,
        XinErrorOut::not_implemented("command not implemented yet"),
    )
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
