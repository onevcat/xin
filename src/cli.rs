use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "xin", version, about = "Agent-first JMAP CLI")]
pub struct Cli {
    /// Output JSON to stdout (default).
    #[arg(long, global = true, conflicts_with = "plain")]
    pub json: bool,

    /// Output stable plain text (TBD).
    #[arg(long, global = true, conflicts_with = "json")]
    pub plain: bool,

    /// Skip confirmations for destructive commands.
    #[arg(long, global = true)]
    pub force: bool,

    /// Never prompt; fail instead.
    #[arg(long, global = true)]
    pub no_input: bool,

    /// Show intended changes without applying.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Choose a configured account (when multiple).
    #[arg(long, global = true)]
    pub account: Option<String>,

    /// Verbose logging.
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Search (thread-like by default).
    Search(SearchArgs),

    /// Per-email search commands.
    Messages {
        #[command(subcommand)]
        command: MessagesCommand,
    },

    /// Get a single email.
    Get(GetArgs),

    /// Thread operations.
    Thread {
        #[command(subcommand)]
        command: ThreadCommand,
    },

    /// Download an attachment.
    Attachment(AttachmentArgs),

    /// Print webmail URL(s) if supported (TBD).
    Url(UrlArgs),

    /// Archive emails.
    Archive(ArchiveArgs),

    /// Mark emails as read.
    Read(ReadArgs),

    /// Mark emails as unread.
    Unread(UnreadArgs),

    /// Move emails to trash.
    Trash(TrashArgs),

    /// Batch operations.
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },

    /// Inbox-zero helpers.
    Inbox {
        #[command(subcommand)]
        command: InboxCommand,
    },

    /// Labels (mailboxes) operations.
    Labels {
        #[command(subcommand)]
        command: LabelsCommand,
    },

    /// Mailboxes operations (alias of labels).
    Mailboxes {
        #[command(subcommand)]
        command: LabelsCommand,
    },

    /// Identities operations.
    Identities {
        #[command(subcommand)]
        command: IdentitiesCommand,
    },

    /// Send an email.
    Send(SendArgs),

    /// Drafts operations.
    Drafts {
        #[command(subcommand)]
        command: DraftsCommand,
    },

    /// History / changes.
    History(HistoryArgs),

    /// Watch (push/polling) (TBD).
    Watch(WatchArgs),
}

// --- Read

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(value_name = "QUERY", allow_hyphen_values = true)]
    pub query: Option<String>,

    #[arg(long = "max")]
    pub max: Option<usize>,

    #[arg(long)]
    pub page: Option<String>,

    #[arg(long)]
    pub oldest: bool,

    #[arg(long = "filter-json")]
    pub filter_json: Option<String>,

    #[arg(long = "collapse-threads")]
    pub collapse_threads: Option<bool>,

    #[arg(long, value_enum, default_value_t = SortKey::ReceivedAt)]
    pub sort: SortKey,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    ReceivedAt,
}

#[derive(Subcommand, Debug)]
pub enum MessagesCommand {
    Search(MessagesSearchArgs),
}

#[derive(Args, Debug)]
pub struct MessagesSearchArgs {
    #[arg(value_name = "QUERY", allow_hyphen_values = true)]
    pub query: Option<String>,

    #[arg(long = "max")]
    pub max: Option<usize>,

    #[arg(long)]
    pub page: Option<String>,

    #[arg(long = "filter-json")]
    pub filter_json: Option<String>,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    pub email_id: String,

    #[arg(long, value_enum, default_value_t = GetFormat::Metadata)]
    pub format: GetFormat,

    /// Max bytes to fetch per body value (only used by --format full). Default: 262144.
    #[arg(long = "max-body-bytes")]
    pub max_body_bytes: Option<usize>,

    #[arg(long)]
    pub headers: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetFormat {
    Metadata,
    Full,
    Raw,
}

#[derive(Subcommand, Debug)]
pub enum ThreadCommand {
    Get(ThreadGetArgs),
    Attachments(ThreadAttachmentsArgs),
    Modify(ThreadModifyArgs),
    Archive(ThreadArchiveArgs),
    Read(ThreadReadArgs),
    Unread(ThreadUnreadArgs),
    Trash(ThreadTrashArgs),
    Delete(ThreadDeleteArgs),
}

#[derive(Args, Debug)]
pub struct ThreadGetArgs {
    pub thread_id: String,

    #[arg(long)]
    pub full: bool,
}

#[derive(Args, Debug)]
pub struct ThreadAttachmentsArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct ThreadModifyArgs {
    pub thread_id: String,

    #[arg(long = "add")]
    pub add: Vec<String>,

    #[arg(long = "remove")]
    pub remove: Vec<String>,

    #[arg(long)]
    pub add_mailbox: Vec<String>,

    #[arg(long)]
    pub remove_mailbox: Vec<String>,

    #[arg(long)]
    pub add_keyword: Vec<String>,

    #[arg(long)]
    pub remove_keyword: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ThreadArchiveArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct ThreadReadArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct ThreadUnreadArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct ThreadTrashArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct ThreadDeleteArgs {
    pub thread_id: String,
}

#[derive(Args, Debug)]
pub struct AttachmentArgs {
    pub email_id: String,
    pub blob_id: String,

    #[arg(long)]
    pub out: Option<String>,

    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct UrlArgs {
    pub ids: Vec<String>,
}

// --- Organize

#[derive(Args, Debug)]
pub struct ArchiveArgs {
    pub email_ids: Vec<String>,

    #[arg(long)]
    pub whole_thread: bool,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    pub email_ids: Vec<String>,

    #[arg(long)]
    pub whole_thread: bool,
}

#[derive(Args, Debug)]
pub struct UnreadArgs {
    pub email_ids: Vec<String>,

    #[arg(long)]
    pub whole_thread: bool,
}

#[derive(Args, Debug)]
pub struct TrashArgs {
    pub email_ids: Vec<String>,

    #[arg(long)]
    pub whole_thread: bool,
}

#[derive(Subcommand, Debug)]
pub enum BatchCommand {
    Modify(BatchModifyArgs),
    Delete(BatchDeleteArgs),
}

#[derive(Subcommand, Debug)]
pub enum InboxCommand {
    /// Get the next email to process from Inbox.
    Next(InboxNextArgs),

    /// Apply an action to an email (and optionally its whole thread).
    Do(InboxDoArgs),
}

#[derive(Args, Debug)]
pub struct InboxNextArgs {
    /// Include already-seen emails (default: only unread).
    #[arg(long)]
    pub all: bool,

    /// Oldest-first (default: newest-first).
    #[arg(long)]
    pub oldest: bool,

    /// Max number of items to return (default: 1).
    #[arg(long = "max")]
    pub max: Option<usize>,

    /// Additional sugar query appended with AND.
    #[arg(value_name = "QUERY", allow_hyphen_values = true)]
    pub query: Option<String>,

    /// Page token (from meta.nextPage).
    #[arg(long)]
    pub page: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxAction {
    Archive,
    Trash,
    Read,
    Unread,
}

#[derive(Args, Debug)]
pub struct InboxDoArgs {
    pub email_id: String,

    #[arg(value_enum)]
    pub action: InboxAction,

    /// Apply to the whole thread containing the given email.
    #[arg(long)]
    pub whole_thread: bool,
}

#[derive(Args, Debug)]
pub struct BatchModifyArgs {
    pub email_ids: Vec<String>,

    #[arg(long = "add")]
    pub add: Vec<String>,

    #[arg(long = "remove")]
    pub remove: Vec<String>,

    #[arg(long)]
    pub add_mailbox: Vec<String>,

    #[arg(long)]
    pub remove_mailbox: Vec<String>,

    #[arg(long)]
    pub add_keyword: Vec<String>,

    #[arg(long)]
    pub remove_keyword: Vec<String>,
}

#[derive(Args, Debug)]
pub struct BatchDeleteArgs {
    pub email_ids: Vec<String>,
}

// --- Labels / Mailboxes

#[derive(Subcommand, Debug)]
pub enum LabelsCommand {
    List(LabelsListArgs),
    Get(LabelsGetArgs),
    Create(LabelsCreateArgs),
    Rename(LabelsRenameArgs),
    Delete(LabelsDeleteArgs),
    Modify(LabelsModifyArgs),
}

#[derive(Args, Debug)]
pub struct LabelsListArgs {}

#[derive(Args, Debug)]
pub struct LabelsGetArgs {
    pub mailbox: String,
}

#[derive(Args, Debug)]
pub struct LabelsCreateArgs {
    pub name: String,

    #[arg(long)]
    pub parent: Option<String>,

    #[arg(long)]
    pub role: Option<String>,

    #[arg(long)]
    pub subscribe: Option<bool>,
}

#[derive(Args, Debug)]
pub struct LabelsRenameArgs {
    pub mailbox_id: String,

    #[arg(long)]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct LabelsDeleteArgs {
    pub mailbox_id: String,

    #[arg(long)]
    pub remove_emails: bool,
}

#[derive(Args, Debug)]
pub struct LabelsModifyArgs {
    pub mailbox_id: String,

    #[arg(long)]
    pub name: Option<String>,

    #[arg(long)]
    pub parent: Option<String>,

    #[arg(long = "sort-order")]
    pub sort_order: Option<i32>,

    #[arg(long)]
    pub subscribe: Option<bool>,
}

// --- Write

#[derive(Subcommand, Debug)]
pub enum IdentitiesCommand {
    List,
    Get(IdentitiesGetArgs),
}

#[derive(Args, Debug)]
pub struct IdentitiesGetArgs {
    pub id: String,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Recipient(s). Can be specified multiple times.
    #[arg(long, required = true, num_args = 1..)]
    pub to: Vec<String>,

    #[arg(long)]
    pub subject: String,

    /// Plain text body. Supports @/path/to/file.txt
    #[arg(long)]
    pub text: Option<String>,

    /// HTML body. Supports @/path/to/file.html
    #[arg(long = "body-html")]
    pub body_html: Option<String>,

    #[arg(long)]
    pub cc: Vec<String>,

    #[arg(long)]
    pub bcc: Vec<String>,

    /// Add attachment(s) by local file path.
    #[arg(long = "attach")]
    pub attach: Vec<String>,

    /// Identity to send as (id or email).
    #[arg(long)]
    pub identity: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum DraftsCommand {
    List(DraftsListArgs),
    Get(DraftsGetArgs),
    Create(DraftsCreateArgs),
    Update(DraftsUpdateArgs),
    Delete(DraftsDeleteArgs),
    Send(DraftsSendArgs),
}

#[derive(Args, Debug)]
pub struct DraftsListArgs {
    #[arg(long = "max")]
    pub max: Option<usize>,

    #[arg(long)]
    pub page: Option<String>,
}

#[derive(Args, Debug)]
pub struct DraftsGetArgs {
    pub draft_email_id: String,

    #[arg(long, value_enum, default_value_t = GetFormat::Metadata)]
    pub format: GetFormat,
}

#[derive(Args, Debug)]
pub struct DraftsCreateArgs {
    #[arg(long)]
    pub to: Vec<String>,

    #[arg(long)]
    pub subject: Option<String>,

    #[arg(long)]
    pub body: Option<String>,

    #[arg(long = "body-file")]
    pub body_file: Option<String>,

    #[arg(long = "body-html")]
    pub body_html: Option<String>,

    #[arg(long)]
    pub cc: Vec<String>,

    #[arg(long)]
    pub bcc: Vec<String>,

    #[arg(long = "attach")]
    pub attach: Vec<String>,

    #[arg(long)]
    pub identity: Option<String>,
}

#[derive(Args, Debug)]
pub struct DraftsUpdateArgs {
    pub draft_email_id: String,

    #[arg(long, num_args = 1..)]
    pub to: Option<Vec<String>>,

    #[arg(long)]
    pub subject: Option<String>,

    #[arg(long)]
    pub body: Option<String>,

    #[arg(long = "body-file")]
    pub body_file: Option<String>,

    #[arg(long = "body-html")]
    pub body_html: Option<String>,

    #[arg(long, num_args = 1..)]
    pub cc: Option<Vec<String>>,

    #[arg(long, num_args = 1..)]
    pub bcc: Option<Vec<String>>,

    /// Add attachment(s) by local file path.
    #[arg(long = "attach")]
    pub attach: Vec<String>,

    /// Replace existing attachments (default: append).
    #[arg(long)]
    pub replace_attachments: bool,

    /// Remove all attachments.
    #[arg(long)]
    pub clear_attachments: bool,

    /// Update From identity for this draft (id or email).
    #[arg(long)]
    pub identity: Option<String>,
}

#[derive(Args, Debug)]
pub struct DraftsDeleteArgs {
    pub draft_email_ids: Vec<String>,
}

#[derive(Args, Debug)]
pub struct DraftsSendArgs {
    pub draft_email_id: String,

    #[arg(long)]
    pub identity: Option<String>,
}

// --- History / watch

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(long)]
    pub since: Option<String>,

    #[arg(long = "max")]
    pub max: Option<usize>,

    #[arg(long)]
    pub page: Option<String>,
}

#[derive(Args, Debug)]
pub struct WatchArgs {}
