use jmap_client::core::query::Filter as CoreFilter;
use jmap_client::core::query::QueryResponse;
use jmap_client::core::set::SetObject;
use jmap_client::email;
use jmap_client::email::Email;
use jmap_client::identity;
use jmap_client::mailbox;
use jmap_client::thread;

use crate::error::XinErrorOut;
use crate::jmap::XinJmap;

pub struct SearchResult {
    pub query: QueryResponse,
    pub emails: Vec<Email>,
}

pub struct ThreadResult {
    pub thread_id: String,
    pub email_ids: Vec<String>,
    pub emails: Vec<Email>,
}

pub struct Backend {
    j: XinJmap,
}

#[derive(Debug, Clone)]
pub struct UploadedBlob {
    pub blob_id: String,
    pub content_type: String,
    pub size: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ModifyPlan {
    pub add_mailboxes: Vec<String>,
    pub remove_mailboxes: Vec<String>,
    pub add_keywords: Vec<String>,
    pub remove_keywords: Vec<String>,
    pub replace_mailboxes: Option<Vec<String>>,
}

impl Backend {
    pub async fn connect() -> Result<Self, XinErrorOut> {
        let cfg = crate::config::RuntimeConfig::from_env()?;
        let j = XinJmap::connect(&cfg).await?;
        Ok(Self { j })
    }

    pub async fn download_blob(&self, blob_id: &str) -> Result<Vec<u8>, XinErrorOut> {
        self.j
            .client()
            .download(blob_id)
            .await
            .map_err(|e| XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("download failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn get_email(
        &self,
        email_id: &str,
        properties: Option<Vec<jmap_client::email::Property>>,
    ) -> Result<Option<Email>, XinErrorOut> {
        self.j
            .client()
            .email_get(email_id, properties)
            .await
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/get failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn get_email_full(
        &self,
        email_id: &str,
        max_body_value_bytes: usize,
        extra_properties: Vec<jmap_client::email::Property>,
    ) -> Result<Option<Email>, XinErrorOut> {
        let mut request = self.j.client().build();

        let get_request = request.get_email().ids([email_id]);

        let mut props: Vec<jmap_client::email::Property> = vec![
            jmap_client::email::Property::Id,
            jmap_client::email::Property::ThreadId,
            jmap_client::email::Property::ReceivedAt,
            jmap_client::email::Property::Subject,
            jmap_client::email::Property::From,
            jmap_client::email::Property::To,
            jmap_client::email::Property::Cc,
            jmap_client::email::Property::Bcc,
            jmap_client::email::Property::Preview,
            jmap_client::email::Property::HasAttachment,
            jmap_client::email::Property::MailboxIds,
            jmap_client::email::Property::Keywords,
            jmap_client::email::Property::BodyStructure,
            jmap_client::email::Property::BodyValues,
            jmap_client::email::Property::TextBody,
            jmap_client::email::Property::HtmlBody,
            jmap_client::email::Property::Attachments,
        ];

        for ep in extra_properties {
            if !props.contains(&ep) {
                props.push(ep);
            }
        }

        get_request.properties(props);

        get_request.arguments().body_properties([
            jmap_client::email::BodyProperty::PartId,
            jmap_client::email::BodyProperty::BlobId,
            jmap_client::email::BodyProperty::Size,
            jmap_client::email::BodyProperty::Name,
            jmap_client::email::BodyProperty::Type,
            jmap_client::email::BodyProperty::Disposition,
            jmap_client::email::BodyProperty::Cid,
        ]);
        get_request
            .arguments()
            .fetch_text_body_values(true)
            .fetch_html_body_values(true)
            .max_body_value_bytes(max_body_value_bytes);

        request
            .send_single::<jmap_client::core::response::EmailGetResponse>()
            .await
            .map(|mut r| r.take_list().pop())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/get(full) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn thread_get(
        &self,
        thread_id: &str,
        include_attachments: bool,
        include_body: bool,
        max_body_value_bytes: usize,
    ) -> Result<Option<ThreadResult>, XinErrorOut> {
        let mut req = self.j.client().build();

        let t = req.get_thread();
        t.ids([thread_id]);
        t.properties([thread::Property::Id, thread::Property::EmailIds]);
        let email_ids_ref = t.result_reference(thread::Property::EmailIds);

        let g = req.get_email();
        g.ids_ref(email_ids_ref);
        let mut props = vec![
            jmap_client::email::Property::Id,
            jmap_client::email::Property::ThreadId,
            jmap_client::email::Property::ReceivedAt,
            jmap_client::email::Property::Subject,
            jmap_client::email::Property::From,
            jmap_client::email::Property::To,
            jmap_client::email::Property::Cc,
            jmap_client::email::Property::Bcc,
            jmap_client::email::Property::Preview,
            jmap_client::email::Property::HasAttachment,
            jmap_client::email::Property::MailboxIds,
            jmap_client::email::Property::Keywords,
        ];

        if include_body {
            props.extend([
                jmap_client::email::Property::BodyStructure,
                jmap_client::email::Property::BodyValues,
                jmap_client::email::Property::TextBody,
                jmap_client::email::Property::HtmlBody,
                jmap_client::email::Property::Attachments,
            ]);

            g.arguments().body_properties([
                jmap_client::email::BodyProperty::PartId,
                jmap_client::email::BodyProperty::BlobId,
                jmap_client::email::BodyProperty::Size,
                jmap_client::email::BodyProperty::Name,
                jmap_client::email::BodyProperty::Type,
                jmap_client::email::BodyProperty::Disposition,
                jmap_client::email::BodyProperty::Cid,
            ]);
            g.arguments()
                .fetch_text_body_values(true)
                .fetch_html_body_values(true)
                .max_body_value_bytes(max_body_value_bytes);
        } else if include_attachments {
            props.push(jmap_client::email::Property::Attachments);
            g.arguments().body_properties([
                jmap_client::email::BodyProperty::BlobId,
                jmap_client::email::BodyProperty::Size,
                jmap_client::email::BodyProperty::Name,
                jmap_client::email::BodyProperty::Type,
                jmap_client::email::BodyProperty::Disposition,
            ]);
        }

        g.properties(props);

        let response = req.send().await.map_err(|e| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: format!("request failed: {e}"),
            http: None,
            jmap: None,
        })?;

        let mut thread_resp: Option<jmap_client::core::response::ThreadGetResponse> = None;
        let mut email_resp: Option<jmap_client::core::response::EmailGetResponse> = None;

        for mr in response.unwrap_method_responses() {
            if mr.is_type(jmap_client::Method::GetThread) {
                if let Ok(r) = mr.unwrap_get_thread() {
                    thread_resp = Some(r);
                }
                continue;
            }
            if mr.is_type(jmap_client::Method::GetEmail) {
                if let Ok(r) = mr.unwrap_get_email() {
                    email_resp = Some(r);
                }
                continue;
            }
        }

        let mut thread_resp = thread_resp.ok_or_else(|| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: "missing Thread/get response".to_string(),
            http: None,
            jmap: None,
        })?;

        let t = match thread_resp.take_list().into_iter().next() {
            Some(t) => t,
            None => return Ok(None),
        };

        let email_ids = t
            .email_ids()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let mut email_resp = email_resp.ok_or_else(|| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: "missing Email/get response".to_string(),
            http: None,
            jmap: None,
        })?;

        Ok(Some(ThreadResult {
            thread_id: t.id().to_string(),
            email_ids,
            emails: email_resp.take_list(),
        }))
    }

    pub async fn search(
        &self,
        filter: Option<CoreFilter<email::query::Filter>>,
        position: i32,
        limit: usize,
        collapse_threads: bool,
        is_ascending: bool,
    ) -> Result<SearchResult, XinErrorOut> {
        let mut req = self.j.client().build();

        let q = req.query_email();
        if let Some(f) = filter {
            q.filter(f);
        }
        q.sort([email::query::Comparator::received_at().is_ascending(is_ascending)]);
        q.limit(limit);
        q.position(position);
        q.arguments().collapse_threads(collapse_threads);

        let ids_ref = q.result_reference();

        let g = req.get_email();
        g.ids_ref(ids_ref);
        g.properties([
            jmap_client::email::Property::Id,
            jmap_client::email::Property::ThreadId,
            jmap_client::email::Property::ReceivedAt,
            jmap_client::email::Property::Subject,
            jmap_client::email::Property::From,
            jmap_client::email::Property::To,
            jmap_client::email::Property::Preview,
            jmap_client::email::Property::HasAttachment,
            jmap_client::email::Property::MailboxIds,
            jmap_client::email::Property::Keywords,
        ]);

        let response = req.send().await.map_err(|e| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: format!("request failed: {e}"),
            http: None,
            jmap: None,
        })?;

        let mut query_resp: Option<QueryResponse> = None;
        let mut get_resp: Option<jmap_client::core::response::EmailGetResponse> = None;

        for mr in response.unwrap_method_responses() {
            if mr.is_type(jmap_client::Method::QueryEmail) {
                if let Ok(r) = mr.unwrap_query_email() {
                    query_resp = Some(r);
                }
                continue;
            }

            if mr.is_type(jmap_client::Method::GetEmail) {
                if let Ok(r) = mr.unwrap_get_email() {
                    get_resp = Some(r);
                }
                continue;
            }
        }

        let query = query_resp.ok_or_else(|| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: "missing Email/query response".to_string(),
            http: None,
            jmap: None,
        })?;

        let mut get_resp = get_resp.ok_or_else(|| XinErrorOut {
            kind: "jmapRequestError".to_string(),
            message: "missing Email/get response".to_string(),
            http: None,
            jmap: None,
        })?;

        Ok(SearchResult {
            query,
            emails: get_resp.take_list(),
        })
    }

    fn mailbox_properties() -> [mailbox::Property; 10] {
        [
            mailbox::Property::Id,
            mailbox::Property::Name,
            mailbox::Property::Role,
            mailbox::Property::ParentId,
            mailbox::Property::SortOrder,
            mailbox::Property::TotalEmails,
            mailbox::Property::UnreadEmails,
            mailbox::Property::TotalThreads,
            mailbox::Property::UnreadThreads,
            mailbox::Property::IsSubscribed,
        ]
    }

    pub async fn list_mailboxes(&self) -> Result<Vec<jmap_client::mailbox::Mailbox>, XinErrorOut> {
        let mut request = self.j.client().build();
        let get_request = request.get_mailbox();
        get_request.properties(Self::mailbox_properties());
        request
            .send_single::<jmap_client::core::response::MailboxGetResponse>()
            .await
            .map(|mut r| r.take_list())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/get failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn get_mailbox(
        &self,
        mailbox_id: &str,
    ) -> Result<Option<jmap_client::mailbox::Mailbox>, XinErrorOut> {
        let mut request = self.j.client().build();
        let get_request = request.get_mailbox();
        get_request.ids([mailbox_id]);
        get_request.properties(Self::mailbox_properties());
        request
            .send_single::<jmap_client::core::response::MailboxGetResponse>()
            .await
            .map(|mut r| r.take_list().pop())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/get failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn create_mailbox(
        &self,
        name: String,
        parent_id: Option<String>,
        role: Option<mailbox::Role>,
        is_subscribed: Option<bool>,
    ) -> Result<serde_json::Value, XinErrorOut> {
        let mut request = self.j.client().build();
        let create = request.set_mailbox().create();
        create.name(name.clone());
        if let Some(pid) = parent_id {
            create.parent_id(Some(pid));
        }
        if let Some(r) = role {
            create.role(r);
        }
        if let Some(s) = is_subscribed {
            create.is_subscribed(s);
        }

        let create_id = create.create_id().unwrap_or_else(|| "c0".to_string());

        request
            .send_single::<jmap_client::core::response::MailboxSetResponse>()
            .await
            .and_then(|mut r| r.created(&create_id))
            .map(|mb| {
                serde_json::json!({
                    "id": mb.id().unwrap_or_default(),
                    "name": mb.name().map(|s| s.to_string()).unwrap_or(name)
                })
            })
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/set(create) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn rename_mailbox(&self, mailbox_id: &str, name: &str) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        request.set_mailbox().update(mailbox_id).name(name.to_string());

        request
            .send_single::<jmap_client::core::response::MailboxSetResponse>()
            .await
            .and_then(|mut r| r.updated(mailbox_id).map(|_| ()))
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/set(update) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn modify_mailbox(
        &self,
        mailbox_id: &str,
        name: Option<String>,
        parent_id: Option<String>,
        sort_order: Option<u32>,
        is_subscribed: Option<bool>,
    ) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        let update = request.set_mailbox().update(mailbox_id);

        if let Some(name) = name {
            update.name(name);
        }
        if let Some(pid) = parent_id {
            update.parent_id(Some(pid));
        }
        if let Some(s) = sort_order {
            update.sort_order(s);
        }
        if let Some(sub) = is_subscribed {
            update.is_subscribed(sub);
        }

        request
            .send_single::<jmap_client::core::response::MailboxSetResponse>()
            .await
            .and_then(|mut r| r.updated(mailbox_id).map(|_| ()))
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/set(update) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn destroy_mailbox(
        &self,
        mailbox_id: &str,
        remove_emails: bool,
    ) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        request
            .set_mailbox()
            .destroy([mailbox_id])
            .arguments()
            .on_destroy_remove_emails(remove_emails);

        request
            .send_single::<jmap_client::core::response::MailboxSetResponse>()
            .await
            .and_then(|mut r| r.destroyed(mailbox_id).map(|_| ()))
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Mailbox/set(destroy) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn list_identities(
        &self,
    ) -> Result<Vec<jmap_client::identity::Identity>, XinErrorOut> {
        let mut request = self.j.client().build();
        let get_request = request.get_identity();
        get_request.properties([
            identity::Property::Id,
            identity::Property::Name,
            identity::Property::Email,
        ]);
        request
            .send_single::<jmap_client::core::response::IdentityGetResponse>()
            .await
            .map(|mut r| r.take_list())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Identity/get failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn upload_blob(
        &self,
        bytes: Vec<u8>,
        content_type: Option<&str>,
        name: Option<String>,
    ) -> Result<UploadedBlob, XinErrorOut> {
        let r = self
            .j
            .client()
            .upload(None, bytes, content_type)
            .await
            .map_err(|e| XinErrorOut {
                kind: "httpError".to_string(),
                message: format!("upload failed: {e}"),
                http: None,
                jmap: None,
            })?;

        Ok(UploadedBlob {
            blob_id: r.blob_id().to_string(),
            content_type: r.content_type().to_string(),
            size: r.size(),
            name,
        })
    }

    pub async fn create_draft_email(
        &self,
        mailbox_id: &str,
        from_name: Option<String>,
        from_email: String,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: Option<&str>,
        text: Option<&str>,
        html: Option<&str>,
        attachments: &[UploadedBlob],
    ) -> Result<Email, XinErrorOut> {
        let mut request = self.j.client().build();
        let create = request.set_email().create();
        create.mailbox_ids([mailbox_id.to_string()]);
        create.keyword("$draft", true);

        if let Some(name) = from_name {
            create.from([(name, from_email.clone())]);
        } else {
            create.from([from_email.clone()]);
        }

        if !to.is_empty() {
            create.to(to.iter().map(|addr| addr.as_str()));
        }
        if !cc.is_empty() {
            create.cc(cc.iter().map(|addr| addr.as_str()));
        }
        if !bcc.is_empty() {
            create.bcc(bcc.iter().map(|addr| addr.as_str()));
        }
        if let Some(s) = subject {
            create.subject(s.to_string());
        }

        let (root, body_values) = build_email_body(text, html, attachments);

        create.body_structure(root.into());
        for (id, value) in body_values {
            create.body_value(id, value);
        }

        let create_id = create
            .create_id()
            .ok_or_else(|| XinErrorOut::config("Email/set missing create id".to_string()))?;

        request
            .send_single::<jmap_client::core::response::EmailSetResponse>()
            .await
            .and_then(|mut r| r.created(&create_id))
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/set failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn update_draft_email(
        &self,
        email_id: &str,
        from_name: Option<String>,
        from_email: Option<String>,
        to: Option<&[String]>,
        cc: Option<&[String]>,
        bcc: Option<&[String]>,
        subject: Option<&str>,
        text: Option<&str>,
        html: Option<&str>,
        attachments: Option<&[UploadedBlob]>,
    ) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        let update = request.set_email().update(email_id.to_string());

        // Ensure it remains a draft.
        update.keyword("$draft", true);

        if let Some(email) = from_email {
            if let Some(name) = from_name {
                update.from([(name, email)]);
            } else {
                update.from([email]);
            }
        }

        if let Some(to) = to {
            update.to(to.iter().map(|addr| addr.as_str()));
        }
        if let Some(cc) = cc {
            update.cc(cc.iter().map(|addr| addr.as_str()));
        }
        if let Some(bcc) = bcc {
            update.bcc(bcc.iter().map(|addr| addr.as_str()));
        }

        if let Some(s) = subject {
            update.subject(s.to_string());
        }

        if text.is_some() || html.is_some() || attachments.is_some() {
            let atts: &[UploadedBlob] = attachments.unwrap_or(&[]);
            let (root, body_values) = build_email_body(text, html, atts);

            update.body_structure(root.into());
            for (id, value) in body_values {
                update.body_value(id, value);
            }
        }

        request
            .send_single::<jmap_client::core::response::EmailSetResponse>()
            .await
            .and_then(|mut r| r.updated(email_id).map(|_| ()))
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/set(update) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn modify_emails(
        &self,
        email_ids: &[String],
        plan: &ModifyPlan,
    ) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        let set = request.set_email();

        for id in email_ids {
            let update = set.update(id.to_string());

            if let Some(repl) = &plan.replace_mailboxes {
                update.mailbox_ids(repl.clone());
            } else {
                for mb in &plan.add_mailboxes {
                    update.mailbox_id(mb, true);
                }
                for mb in &plan.remove_mailboxes {
                    update.mailbox_id(mb, false);
                }
            }

            for kw in &plan.add_keywords {
                update.keyword(kw, true);
            }
            for kw in &plan.remove_keywords {
                update.keyword(kw, false);
            }
        }

        request
            .send_single::<jmap_client::core::response::EmailSetResponse>()
            .await
            .map(|_| ())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/set(update) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn thread_email_ids(
        &self,
        thread_id: &str,
    ) -> Result<Option<Vec<String>>, XinErrorOut> {
        let mut request = self.j.client().build();
        let t = request.get_thread();
        t.ids([thread_id]);
        t.properties([thread::Property::Id, thread::Property::EmailIds]);

        request
            .send_single::<jmap_client::core::response::ThreadGetResponse>()
            .await
            .map(|mut r| {
                r.take_list().into_iter().next().map(|t| {
                    t.email_ids()
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                })
            })
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Thread/get failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn destroy_emails(&self, email_ids: &[String]) -> Result<(), XinErrorOut> {
        let mut request = self.j.client().build();
        request.set_email().destroy(email_ids.iter().map(|s| s.as_str()));

        request
            .send_single::<jmap_client::core::response::EmailSetResponse>()
            .await
            .map(|_| ())
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("Email/set(destroy) failed: {e}"),
                http: None,
                jmap: None,
            })
    }

    pub async fn submit_email(
        &self,
        email_id: &str,
        identity_id: &str,
    ) -> Result<jmap_client::email_submission::EmailSubmission, XinErrorOut> {
        self.j
            .client()
            .email_submission_create(email_id, identity_id)
            .await
            .map_err(|e| XinErrorOut {
                kind: "jmapRequestError".to_string(),
                message: format!("EmailSubmission/set failed: {e}"),
                http: None,
                jmap: None,
            })
    }
}

fn build_email_body(
    text: Option<&str>,
    html: Option<&str>,
    attachments: &[UploadedBlob],
) -> (
    jmap_client::email::EmailBodyPart<jmap_client::Set>,
    Vec<(String, String)>,
) {
    type PartSet = jmap_client::email::EmailBodyPart<jmap_client::Set>;

    let mut body_values: Vec<(String, String)> = Vec::new();

    let body_part: Option<PartSet> = match (text, html) {
        (Some(t), Some(h)) => {
            let alt = jmap_client::email::EmailBodyPart::new()
                .content_type("multipart/alternative")
                .sub_part(
                    jmap_client::email::EmailBodyPart::new()
                        .part_id("text")
                        .content_type("text/plain")
                        .into(),
                )
                .sub_part(
                    jmap_client::email::EmailBodyPart::new()
                        .part_id("html")
                        .content_type("text/html")
                        .into(),
                );

            body_values.push(("text".to_string(), t.to_string()));
            body_values.push(("html".to_string(), h.to_string()));

            Some(alt)
        }
        (Some(t), None) => {
            body_values.push(("text".to_string(), t.to_string()));
            Some(
                jmap_client::email::EmailBodyPart::new()
                    .part_id("text")
                    .content_type("text/plain"),
            )
        }
        (None, Some(h)) => {
            body_values.push(("html".to_string(), h.to_string()));
            Some(
                jmap_client::email::EmailBodyPart::new()
                    .part_id("html")
                    .content_type("text/html"),
            )
        }
        (None, None) => None,
    };

    // If body is omitted (attachments-only draft), create an empty text/plain body.
    if body_part.is_none() {
        body_values.push(("text".to_string(), "".to_string()));
    }

    let root: PartSet = if !attachments.is_empty() {
        let mut mixed = jmap_client::email::EmailBodyPart::new().content_type("multipart/mixed");

        mixed = mixed.sub_part(
            body_part
                .unwrap_or_else(|| {
                    jmap_client::email::EmailBodyPart::new()
                        .part_id("text")
                        .content_type("text/plain")
                })
                .into(),
        );

        for a in attachments {
            let mut p = jmap_client::email::EmailBodyPart::new()
                .blob_id(a.blob_id.clone())
                .content_type(a.content_type.clone());
            if let Some(name) = &a.name {
                p = p.name(name.clone());
            }
            mixed = mixed.sub_part(p.into());
        }
        mixed
    } else {
        body_part.unwrap_or_else(|| {
            jmap_client::email::EmailBodyPart::new()
                .part_id("text")
                .content_type("text/plain")
        })
    };

    (root, body_values)
}

