use jmap_client::core::query::Filter as CoreFilter;
use jmap_client::core::query::QueryResponse;
use jmap_client::email;
use jmap_client::email::Email;

use crate::error::XinErrorOut;
use crate::jmap::XinJmap;

pub struct SearchResult {
    pub query: QueryResponse,
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
