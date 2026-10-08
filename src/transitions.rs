// SPDX-License-Identifier: Apache-2.0
//! Authenticated, resumable activation coordinator; participant calls never hold local locks.
use super::*;
use munarium_council::coordinator::Ratifier;
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Operation {
    Propose { transition: String },
    Ratify { transition_id: String },
    Lookup { transition_id: String },
    Advance { transition_id: String },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    tenant: String,
    action: Operation,
}
async fn admitted(runtime: &Runtime, peer: &Peer, body: &[u8]) -> wire::Result<Value> {
    let _permit = runtime
        .permits
        .try_acquire()
        .map_err(|_| Error::Unavailable)?;
    let request: Request = serde_json::from_slice(body).map_err(|_| Error::Invalid)?;
    if !peer.tenants.contains(&request.tenant) {
        return Err(Error::Refused);
    }
    let state = service_transport::authority(
        &runtime.client,
        &runtime.config.server_endpoint,
        &runtime.config.deployment,
        &request.tenant,
    )
    .await
    .map_err(unavailable)?;
    let policy: Policy = serde_json::from_value(
        state["artifact"]["bindings"][format!("stage2:{}", runtime.config.service)].clone(),
    )
    .map_err(|_| Error::Refused)?;
    if policy.scope["tenant"] != request.tenant
        || policy.scope["deployment"] != runtime.config.deployment
    {
        return Err(Error::Refused);
    }
    let now = service_transport::now().map_err(unavailable)? as u64;
    match request.action {
        Operation::Propose { transition } => {
            let proposer = policy
                .proposers
                .get(&peer.service)
                .or_else(|| policy.humans.get(&peer.service).map(|h| &h.principal))
                .ok_or(Error::Refused)?;
            runtime.coordinator.lock().await.propose(
                &policy.scope,
                proposer,
                &wire::parse(&transition)?,
                now,
            )
        }
        Operation::Ratify { transition_id } => {
            let h = policy.humans.get(&peer.service).ok_or(Error::Refused)?;
            runtime.coordinator.lock().await.ratify(
                &Ratifier {
                    scope: policy.scope.clone(),
                    principal: h.principal.clone(),
                    revision: h.eligibility_revision,
                    now,
                },
                &transition_id,
            )
        }
        Operation::Lookup { transition_id } => {
            if !policy.readers.contains(&peer.service)
                && !policy.humans.contains_key(&peer.service)
                && !policy.proposers.contains_key(&peer.service)
            {
                return Err(Error::Refused);
            }
            let mut result = runtime
                .coordinator
                .lock()
                .await
                .lookup(&policy.scope, &transition_id)?;
            let eligible = policy.humans.values().any(|h| {
                result["ratifier"] == h.principal
                    && result["eligibility_revision"] == h.eligibility_revision
            });
            let t = &result["transition"];
            let fresh = t["not_before"]
                .as_u64()
                .is_some_and(|n| n.saturating_add(2) <= now)
                && t["expires_at"]
                    .as_u64()
                    .is_some_and(|n| now.saturating_add(2) < n);
            if !eligible || !fresh {
                result["ratified"] = json!(false);
            }
            Ok(result)
        }
        Operation::Advance { transition_id } => {
            if !policy.humans.contains_key(&peer.service) {
                return Err(Error::Refused);
            }
            let state = runtime
                .coordinator
                .lock()
                .await
                .lookup(&policy.scope, &transition_id)?;
            if state["ratified"] != true
                || !policy.humans.values().any(|h| {
                    state["ratifier"] == h.principal
                        && state["eligibility_revision"] == h.eligibility_revision
                })
            {
                return Err(Error::Refused);
            }
            let transition = &state["transition"];
            if now < wire::number(&transition["not_before"])?.saturating_add(2)
                || now.saturating_add(2) >= wire::number(&transition["expires_at"])?
            {
                return Err(Error::Refused);
            }
            let gate = format!(
                "{}/v1/actions",
                runtime.config.gate_endpoint.trim_end_matches('/')
            );
            let encoded = wire::canonical(transition)?;
            if state["pause"].is_null() {
                let pause=runtime.post(&gate,json!({"tenant":request.tenant,"action":{"operation":"pause","transition":encoded}})).await?;
                runtime.coordinator.lock().await.receipt(
                    &policy.scope,
                    &transition_id,
                    "gate",
                    &pause,
                )?;
            }
            for participant in ["registry", "server", "warden", "gate"] {
                let state = runtime
                    .coordinator
                    .lock()
                    .await
                    .lookup(&policy.scope, &transition_id)?;
                if state["receipts"]
                    .as_array()
                    .ok_or(Error::Unavailable)?
                    .iter()
                    .any(|r| r["participant"] == participant)
                {
                    continue;
                }
                let endpoint = match participant {
                    "registry" => format!(
                        "{}/v1/activation",
                        runtime
                            .config
                            .registry_endpoint
                            .as_ref()
                            .ok_or(Error::Unavailable)?
                            .trim_end_matches('/')
                    ),
                    "server" => format!(
                        "{}/v1/platform/{}/activation",
                        runtime.config.server_endpoint.trim_end_matches('/'),
                        request.tenant
                    ),
                    "warden" => format!(
                        "{}/v1/activation",
                        runtime.config.warden_endpoint.trim_end_matches('/')
                    ),
                    _ => gate.clone(),
                };
                let receipt=runtime.post(&endpoint,json!({"tenant":request.tenant,"action":{"operation":if participant=="gate"{"apply-activation"}else{"apply"},"transition":encoded}})).await?;
                runtime.coordinator.lock().await.receipt(
                    &policy.scope,
                    &transition_id,
                    participant,
                    &receipt,
                )?;
            }
            let completion = runtime
                .coordinator
                .lock()
                .await
                .completion(&policy.scope, &transition_id)?;
            let reply=runtime.post(&gate,json!({"tenant":request.tenant,"action":{"operation":"resume","completion":completion}})).await?;
            runtime
                .coordinator
                .lock()
                .await
                .resumed(&policy.scope, &transition_id, &reply)?;
            runtime
                .coordinator
                .lock()
                .await
                .lookup(&policy.scope, &transition_id)
        }
    }
}
pub(super) async fn operate(
    State(runtime): State<Arc<Runtime>>,
    ConnectInfo(peer): ConnectInfo<Peer>,
    body: Bytes,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    admitted(&runtime, &peer, &body)
        .await
        .map(Json)
        .map_err(|e| {
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
