use jmap_client::core::query::Filter as CoreFilter;
use jmap_client::core::query::QueryResponse;
use jmap_client::email;
use jmap_client::email::Email;
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
        self.j.client().download(blob_id).await.map_err(|e| XinErrorOut {
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

    pub async fn thread_get(&self, thread_id: &str, include_attachments: bool) -> Result<Option<ThreadResult>, XinErrorOut> {
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

        let email_ids = t.email_ids().iter().map(|s| s.to_string()).collect::<Vec<_>>();

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
        q.sort([
            email::query::Comparator::received_at().is_ascending(is_ascending),
        ]);
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
}
