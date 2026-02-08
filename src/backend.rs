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
    ) -> Result<Option<Email>, XinErrorOut> {
        let mut request = self.j.client().build();

        let get_request = request.get_email().ids([email_id]);
        get_request.properties([
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
        ]);

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

        if include_attachments {
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

    pub async fn list_mailboxes(&self) -> Result<Vec<jmap_client::mailbox::Mailbox>, XinErrorOut> {
        let mut request = self.j.client().build();
        let get_request = request.get_mailbox();
        get_request.properties([
            mailbox::Property::Id,
            mailbox::Property::Name,
            mailbox::Property::Role,
        ]);
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

    pub async fn create_text_email(
        &self,
        mailbox_id: &str,
        from_name: Option<String>,
        from_email: String,
        to: &[String],
        subject: &str,
        text: &str,
    ) -> Result<Email, XinErrorOut> {
        let mut request = self.j.client().build();
        let create = request.set_email().create();
        create.mailbox_ids([mailbox_id.to_string()]);

        if let Some(name) = from_name {
            create.from([(name, from_email.clone())]);
        } else {
            create.from([from_email.clone()]);
        }
        create.to(to.iter().map(|addr| addr.as_str()));
        create.subject(subject.to_string());

        let part_id = "text";
        let body_part = jmap_client::email::EmailBodyPart::new()
            .part_id(part_id)
            .content_type("text/plain");
        let text_part = jmap_client::email::EmailBodyPart::new()
            .part_id(part_id)
            .content_type("text/plain");
        create.body_structure(body_part.into());
        create.text_body(text_part);
        create.body_value(part_id.to_string(), text);

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
