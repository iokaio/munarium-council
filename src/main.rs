// SPDX-License-Identifier: Apache-2.0
//! Experimental authenticated approval service. No target credential or target dispatch.
#[path = "../vendor/warden-transport/service_transport.rs"]
mod service_transport;
mod transitions;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use munarium_council::{
    store::{Authority, Store},
    wire::{self, Error},
};
use serde::Deserialize;
use serde_json::{Value, json};
use service_transport::{Peer, TlsConfig};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    tls: TlsConfig,
    database: PathBuf,
    server_endpoint: String,
    gate_endpoint: String,
    warden_endpoint: String,
    deployment: String,
    service: String,
    server_service: String,
    provider_id: String,
    provider_token_file: PathBuf,
    registry_endpoint: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Human {
    principal: Value,
    eligibility_revision: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    scope: Value,
    humans: BTreeMap<String, Human>,
    readers: BTreeSet<String>,
    stream: String,
    generation: u64,
    #[serde(default)]
    proposers: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Operation {
    Approve {
        id: String,
        operation_id: String,
        attempt_id: String,
    },
    Lookup {
        id: String,
    },
    Withdraw {
        id: String,
        withdrawal_id: String,
    },
    Flush,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    tenant: String,
    action: Operation,
}
struct Runtime {
    config: Config,
    client: reqwest::Client,
    store: tokio::sync::Mutex<Store>,
    permits: tokio::sync::Semaphore,
    coordinator: tokio::sync::Mutex<munarium_council::coordinator::Coordinator>,
}
fn unavailable(_: service_transport::Failure) -> Error {
    Error::Unavailable
}
impl Runtime {
    async fn post(&self, endpoint: &str, body: Value) -> wire::Result<Value> {
        let response = self
            .client
            .post(endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|_| Error::Unavailable)?;
        service_transport::json_response(response, 1048576)
            .await
            .map_err(unavailable)
    }
    async fn source(&self, tenant: &str, operation: &str, attempt: &str) -> wire::Result<Value> {
        self.post(&format!("{}/v1/actions",self.config.gate_endpoint.trim_end_matches('/')),json!({"tenant":tenant,"action":{"operation":"source","operation_id":operation,"attempt_id":attempt}})).await
    }
    async fn assertion(&self, tenant: &str) -> wire::Result<Value> {
        let token = std::fs::read_to_string(&self.config.provider_token_file)
            .map_err(|_| Error::Unavailable)?;
        if token.len() > 65536 {
            return Err(Error::Unavailable);
        }
        let response=self.post(&format!("{}/v1/identity",self.config.warden_endpoint.trim_end_matches('/')),json!({"tenant":tenant,"audience":self.config.server_service,"action":{"operation":"root","provider_id":self.config.provider_id,"token":token.trim(),"scopes":["read","record"],"resources":[format!("action-records:{tenant}")]}})).await?;
        let usable = response["usable_at"].as_i64().ok_or(Error::Unavailable)?;
        let now = service_transport::now().map_err(unavailable)?;
        if usable > now + 3 {
            return Err(Error::Unavailable);
        }
        if usable >= now {
            tokio::time::sleep(std::time::Duration::from_secs((usable - now + 1) as u64)).await;
        }
        if !response["chain"].is_array() {
            return Err(Error::Unavailable);
        }
        Ok(response["chain"].clone())
    }
    async fn flush(&self, tenant: &str, scope: &Value) -> wire::Result<Value> {
        let events = self.store.lock().await.pending(scope)?;
        if events.is_empty() {
            return Ok(json!({"pending":0}));
        }
        let chain = self.assertion(tenant).await?;
        for event in events {
            let id = event["payload"]["approval"]["id"]
                .as_str()
                .ok_or(Error::Unavailable)?;
            let record = self.store.lock().await.issuance(scope, id)?;
            let endpoint = format!(
                "{}/v1/platform/{tenant}/records",
                self.config.server_endpoint.trim_end_matches('/')
            );
            self.post(&endpoint,json!({"chain":chain,"action":{"operation":"action-archive","record":wire::canonical(&record)?}})).await?;
            let ack=self.post(&endpoint,json!({"chain":chain,"action":{"operation":"action-append","event":wire::canonical(&event)?}})).await?;
            self.store.lock().await.acknowledge(scope, &event, &ack)?;
        }
        Ok(json!({"pending":self.store.lock().await.pending(scope)?.len()}))
    }
    async fn operate(&self, peer: &Peer, body: &[u8]) -> wire::Result<Value> {
        let _permit = self.permits.try_acquire().map_err(|_| Error::Unavailable)?;
        let request: Request = serde_json::from_slice(body).map_err(|_| Error::Invalid)?;
        if !peer.tenants.contains(&request.tenant) {
            return Err(Error::Refused);
        }
        let state = service_transport::authority(
            &self.client,
            &self.config.server_endpoint,
            &self.config.deployment,
            &request.tenant,
        )
        .await
        .map_err(unavailable)?;
        let policy: Policy = serde_json::from_value(
            state["artifact"]["bindings"][format!("stage2:{}", self.config.service)].clone(),
        )
        .map_err(|_| Error::Refused)?;
        if policy.scope["tenant"] != request.tenant
            || policy.scope["deployment"] != self.config.deployment
        {
            return Err(Error::Refused);
        }
        let human = policy.humans.get(&peer.service);
        let reader = policy.readers.contains(&peer.service);
        if human.is_none() && !reader {
            return Err(Error::Refused);
        }
        match request.action {
            Operation::Approve {
                id,
                operation_id,
                attempt_id,
            } => {
                let human = human.ok_or(Error::Refused)?;
                let source = self
                    .source(&request.tenant, &operation_id, &attempt_id)
                    .await?;
                if source["request"]["operation"]["id"] != operation_id
                    || source["request"]["attempt"]["id"] != attempt_id
                {
                    return Err(Error::Refused);
                }
                let auth = authority(&policy, human, &source)?;
                self.store
                    .lock()
                    .await
                    .approve(&auth, &source["request"], &source["decision"], &id)
            }
            Operation::Lookup { id } => {
                let record = self.store.lock().await.issuance(&policy.scope, &id)?;
                if human.is_some_and(|h| h.principal != record["approver"]) && !reader {
                    return Err(Error::Refused);
                }
                let source = self
                    .source(
                        &request.tenant,
                        record["operation"]["id"].as_str().ok_or(Error::Invalid)?,
                        record["attempt"]["id"].as_str().ok_or(Error::Invalid)?,
                    )
                    .await?;
                let active = policy
                    .humans
                    .values()
                    .find(|h| h.principal == record["approver"]);
                if let Some(active) = active {
                    let auth = authority(&policy, active, &source)?;
                    self.store.lock().await.lookup(&auth, &id)
                } else {
                    Ok(json!({"approval":record,"status":"ineligible","currently_usable":false}))
                }
            }
            Operation::Withdraw { id, withdrawal_id } => {
                let human = human.ok_or(Error::Refused)?;
                let record = self.store.lock().await.issuance(&policy.scope, &id)?;
                if record["approver"] != human.principal {
                    return Err(Error::Refused);
                }
                let pending =
                    self.store
                        .lock()
                        .await
                        .withdraw(&policy.scope, &id, &withdrawal_id)?;
                if pending["status"] != "withdrawal-pending" {
                    return Ok(pending);
                }
                let approval_revision = pending["revision"]
                    .as_u64()
                    .and_then(|r| r.checked_sub(1))
                    .ok_or(Error::Unavailable)?;
                let receipt=self.post(&format!("{}/v1/actions",self.config.gate_endpoint.trim_end_matches('/')),json!({"tenant":request.tenant,"action":{"operation":"cancel","approval":record["approval"],"approval_revision":approval_revision,"operation_ref":record["operation"],"attempt":record["attempt"],"withdrawal_id":withdrawal_id}})).await?;
                self.store
                    .lock()
                    .await
                    .finish_withdrawal(&policy.scope, &id, &receipt)?;
                self.store
                    .lock()
                    .await
                    .withdraw(&policy.scope, &id, &withdrawal_id)
            }
            Operation::Flush => {
                if !reader {
                    return Err(Error::Refused);
                }
                self.flush(&request.tenant, &policy.scope).await
            }
        }
    }
}
fn authority(policy: &Policy, human: &Human, source: &Value) -> wire::Result<Authority> {
    let requester_chain =
        serde_json::from_value(source["requester_chain"].clone()).map_err(|_| Error::Refused)?;
    Ok(Authority {
        scope: policy.scope.clone(),
        approver: human.principal.clone(),
        eligibility_revision: human.eligibility_revision,
        requester_chain,
        context: source["current_context"].clone(),
        stream: policy.stream.clone(),
        generation: policy.generation,
        now: service_transport::now()
            .map_err(unavailable)?
            .try_into()
            .map_err(|_| Error::Unavailable)?,
    })
}
async fn operate(
    State(runtime): State<Arc<Runtime>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    runtime.operate(&peer, &body).await.map(Json).map_err(|e| {
        (
            match e {
                Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                Error::Conflict => StatusCode::CONFLICT,
                _ => StatusCode::FORBIDDEN,
            },
            Json(json!({"error":e.to_string()})),
        )
    })
}
async fn run() -> wire::Result<()> {
    let path = std::env::args_os().nth(1).ok_or(Error::Invalid)?;
    let raw = std::fs::read(path).map_err(|_| Error::Invalid)?;
    if raw.len() > 1048576 {
        return Err(Error::Invalid);
    }
    let config: Config = serde_json::from_slice(&raw).map_err(|_| Error::Invalid)?;
    if !config.database.is_absolute()
        || [
            &config.server_endpoint,
            &config.gate_endpoint,
            &config.warden_endpoint,
        ]
        .iter()
        .any(|v| !v.starts_with("https://"))
    {
        return Err(Error::Invalid);
    }
    let store = Store::open(&config.database)?;
    let coordinator = munarium_council::coordinator::Coordinator::open(&config.database)?;
    let client = service_transport::client(&config.tls).map_err(unavailable)?;
    let listener = service_transport::Mtls::bind(&config.tls)
        .await
        .map_err(unavailable)?;
    let runtime = Arc::new(Runtime {
        config,
        client,
        store: tokio::sync::Mutex::new(store),
        permits: tokio::sync::Semaphore::new(32),
        coordinator: tokio::sync::Mutex::new(coordinator),
    });
    let app = Router::new()
        .route("/v1/approvals", post(operate))
        .route("/v1/transitions", post(transitions::operate))
        .layer(DefaultBodyLimit::max(65536))
        .with_state(runtime);
    axum::serve(listener, app.into_make_service_with_connect_info::<Peer>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|_| Error::Unavailable)
}
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("Council unavailable");
        std::process::exit(1);
    }
}
