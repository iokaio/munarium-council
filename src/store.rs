// SPDX-License-Identifier: Apache-2.0
//! Transactional approval custody. Authentication belongs to the service adapter.
use crate::wire::{self, Error, Result};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::path::Path;

/// Independently authenticated, current inputs, never deserialized from an approval request.
pub struct Authority {
    /// Exact admitted tenant/deployment/cell scope.
    pub scope: Value,
    /// Operator-enrolled human principal.
    pub approver: Value,
    /// Current enrollment revision.
    pub eligibility_revision: u64,
    /// Verified complete requester ancestry from the owning service.
    pub requester_chain: Vec<Value>,
    /// Independently retrieved current decision context.
    pub context: Value,
    /// Registered Council event stream.
    pub stream: String,
    /// Registered Council source generation.
    pub generation: u64,
    /// Current bounded UTC time.
    pub now: u64,
}

/// Durable approval and lifecycle store; each method uses one SQLite transaction.
pub struct Store {
    db: Connection,
}
impl Store {
    /// Retrieve immutable issuance for an already authorized adapter; no current-use claim.
    pub fn issuance(&self, scope: &Value, id: &str) -> Result<Value> {
        let raw: String = self
            .db
            .query_row(
                "SELECT record FROM approvals WHERE scope=?1 AND id=?2",
                params![wire::canonical(scope)?, id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::Refused)?;
        wire::parse(&raw)
    }
    /// Open/create additive tables. Multiple writers serialize through SQLite.
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
          CREATE TABLE IF NOT EXISTS approvals(scope TEXT NOT NULL,id TEXT NOT NULL,operation TEXT NOT NULL,attempt TEXT NOT NULL,binding TEXT NOT NULL,record TEXT NOT NULL,status TEXT NOT NULL,revision INTEGER NOT NULL,withdrawal TEXT,PRIMARY KEY(scope,id),UNIQUE(scope,operation,attempt));
          CREATE TABLE IF NOT EXISTS history(seq INTEGER PRIMARY KEY,scope TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,state TEXT NOT NULL,detail TEXT NOT NULL,UNIQUE(scope,id,revision));
          CREATE TABLE IF NOT EXISTS outbox(scope TEXT NOT NULL,stream TEXT NOT NULL,generation INTEGER NOT NULL,sequence INTEGER NOT NULL,event TEXT NOT NULL,digest TEXT NOT NULL,ack TEXT,PRIMARY KEY(scope,stream,generation,sequence));")?;
        Ok(Self { db })
    }

    /// Create one approval or recover an exact immutable retry without renewing expiry.
    pub fn approve(
        &mut self,
        auth: &Authority,
        request: &Value,
        decision: &Value,
        id: &str,
    ) -> Result<Value> {
        validate_inputs(auth, request, decision)?;
        let scope = wire::canonical(&auth.scope)?;
        let operation = wire::canonical(&request["operation"])?;
        let attempt = wire::canonical(&request["attempt"])?;
        let binding = wire::digest(
            "approval-intake",
            &json!({"request":request,"decision":decision,"approver":auth.approver,"eligibility_revision":auth.eligibility_revision}),
        )?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, String, String, u64)> = tx
            .query_row(
                "SELECT binding,record,status,revision FROM approvals WHERE scope=?1 AND id=?2",
                params![scope, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some((old, record, status, revision)) = prior {
            if old != binding {
                return Err(Error::Conflict);
            }
            let record = wire::parse(&record)?;
            return Ok(view(record, &status, revision, auth));
        }
        if tx.query_row(
            "SELECT COUNT(*) FROM approvals WHERE scope=?1 AND operation=?2 AND attempt=?3",
            params![scope, operation, attempt],
            |r| r.get::<_, u64>(0),
        )? != 0
        {
            return Err(Error::Conflict);
        }
        let expires = auth
            .now
            .checked_add(300)
            .ok_or(Error::Invalid)?
            .min(wire::number(&request["context"]["expires_at"])?);
        if expires <= auth.now.saturating_add(4) {
            return Err(Error::Refused);
        }
        let record = json!({"schema_version":1,"profile":"stage2-single-cell-v1","type":"action-approval",
          "approval":{"id":id,"kind":"approval","scope":auth.scope},"operation":request["operation"],"attempt":request["attempt"],
          "request_digest":wire::digest("action-request",request)?,"context_digest":request["context_digest"],
          "decision_digest":wire::digest("action-decision",decision)?,"approver":auth.approver,"eligibility_revision":auth.eligibility_revision,
          "issued_at":auth.now,"expires_at":expires,"prior":null});
        wire::shape(&record, "action-approval")?;
        let raw = wire::canonical(&record)?;
        tx.execute(
            "INSERT INTO approvals VALUES(?1,?2,?3,?4,?5,?6,'approved',1,NULL)",
            params![scope, id, operation, attempt, binding, raw],
        )?;
        tx.execute(
            "INSERT INTO history(scope,id,revision,state,detail) VALUES(?1,?2,1,'approved',?3)",
            params![scope, id, raw],
        )?;
        let previous: Option<(u64,String)> = tx.query_row("SELECT sequence,digest FROM outbox WHERE scope=?1 AND stream=?2 AND generation=?3 ORDER BY sequence DESC LIMIT 1",params![scope,auth.stream,auth.generation],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let sequence = previous.as_ref().map_or(1, |p| p.0 + 1);
        let payload = json!({"kind":"approval-recorded","operation":request["operation"],"attempt":request["attempt"],"request_digest":record["request_digest"],"context_digest":record["context_digest"],"activation_epoch":auth.context["activation"]["revision"],"recovery_epoch":auth.context["recovery"]["revision"],"approval":record["approval"],"approval_digest":wire::digest("action-approval",&record)?});
        let event = json!({"schema_version":1,"profile":"stage2-single-cell-v1","type":"accountability-event","scope":auth.scope,"event_id":format!("approval-{id}"),"producer":"council","stream":{"id":auth.stream,"kind":"stream","scope":auth.scope},"source_generation":auth.generation,"sequence":sequence,"predecessor":previous.map(|p|p.1),"occurred_at":auth.now,"clock":"bounded-utc-2s","family":"action","kind":"approval-recorded","payload_digest":wire::digest("event-payload",&payload)?,"payload":payload,"causal_parents":[]});
        wire::shape(&event, "accountability-event")?;
        tx.execute(
            "INSERT INTO outbox VALUES(?1,?2,?3,?4,?5,?6,NULL)",
            params![
                scope,
                auth.stream,
                auth.generation,
                sequence,
                wire::canonical(&event)?,
                wire::digest("accountability-event", &event)?
            ],
        )?;
        tx.commit()?;
        Ok(view(record, "approved", 1, auth))
    }

    /// Lookup retained issuance and separately evaluate current eligibility and expiry.
    pub fn lookup(&self, auth: &Authority, id: &str) -> Result<Value> {
        let row: Option<(String, String, u64)> = self
            .db
            .query_row(
                "SELECT record,status,revision FROM approvals WHERE scope=?1 AND id=?2",
                params![wire::canonical(&auth.scope)?, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (record, status, revision) = row.ok_or(Error::Refused)?;
        Ok(view(wire::parse(&record)?, &status, revision, auth))
    }

    /// Begin an idempotent withdrawal; it is ineffective until Gate commits cancellation.
    pub fn withdraw(&mut self, scope: &Value, id: &str, withdrawal: &str) -> Result<Value> {
        if withdrawal.is_empty() || withdrawal.len() > 128 {
            return Err(Error::Invalid);
        }
        let scope = wire::canonical(scope)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (record, status, revision, old): (String, String, u64, Option<String>) = tx
            .query_row(
                "SELECT record,status,revision,withdrawal FROM approvals WHERE scope=?1 AND id=?2",
                params![scope, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
            .ok_or(Error::Refused)?;
        if let Some(old) = old {
            if old != withdrawal {
                return Err(Error::Conflict);
            }
            return Ok(
                json!({"approval":wire::parse(&record)?,"status":status,"revision":revision,"withdrawal_id":old}),
            );
        }
        tx.execute("UPDATE approvals SET status='withdrawal-pending',revision=revision+1,withdrawal=?3 WHERE scope=?1 AND id=?2",params![scope,id,withdrawal])?;
        tx.execute("INSERT INTO history(scope,id,revision,state,detail) VALUES(?1,?2,?3,'withdrawal-pending',?4)",params![scope,id,revision+1,withdrawal])?;
        tx.commit()?;
        Ok(
            json!({"approval":wire::parse(&record)?,"status":"withdrawal-pending","revision":revision+1,"withdrawal_id":withdrawal}),
        )
    }

    /// Finish withdrawal only from an authenticated Gate receipt with exact bindings.
    pub fn finish_withdrawal(&mut self, scope: &Value, id: &str, receipt: &Value) -> Result<()> {
        let scope_key = wire::canonical(scope)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (record, status, revision, withdrawal): (String, String, u64, Option<String>) = tx
            .query_row(
                "SELECT record,status,revision,withdrawal FROM approvals WHERE scope=?1 AND id=?2",
                params![scope_key, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
            .ok_or(Error::Refused)?;
        let record = wire::parse(&record)?;
        if receipt["approval"] != record["approval"]
            || receipt["operation"] != record["operation"]
            || receipt["attempt"] != record["attempt"]
            || withdrawal.as_deref() != receipt["withdrawal_id"].as_str()
            || withdrawal.is_none()
        {
            return Err(Error::Refused);
        }
        let next = match receipt["status"].as_str() {
            Some("cancelled") => "withdrawn",
            Some("too-late") => "withdrawal-too-late",
            _ => return Err(Error::Refused),
        };
        if status == next {
            let stored: String = tx.query_row(
                "SELECT detail FROM history WHERE scope=?1 AND id=?2 AND revision=?3",
                params![scope_key, id, revision],
                |r| r.get(0),
            )?;
            return if wire::parse(&stored)? == *receipt {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        if status != "withdrawal-pending"
            || revision < 2
            || receipt["approval_revision"] != revision - 1
        {
            return Err(Error::Conflict);
        }
        tx.execute(
            "UPDATE approvals SET status=?3,revision=revision+1 WHERE scope=?1 AND id=?2",
            params![scope_key, id, next],
        )?;
        tx.execute(
            "INSERT INTO history(scope,id,revision,state,detail) VALUES(?1,?2,?3,?4,?5)",
            params![scope_key, id, revision + 1, next, wire::canonical(receipt)?],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Read pending immutable audit events in stream order; this grants no send authority.
    pub fn pending(&self, scope: &Value) -> Result<Vec<Value>> {
        let mut stmt=self.db.prepare("SELECT event FROM outbox WHERE scope=?1 AND ack IS NULL ORDER BY generation,sequence LIMIT 100")?;
        let rows = stmt.query_map([wire::canonical(scope)?], |r| r.get::<_, String>(0))?;
        rows.map(|r| wire::parse(&r?)).collect()
    }

    /// Retain an exact authenticated Server acknowledgement, refusing substitutions.
    pub fn acknowledge(&mut self, scope: &Value, event: &Value, ack: &Value) -> Result<()> {
        wire::shape(ack, "event-ack")?;
        wire::scope(ack, scope)?;
        if ack["event_digest"] != wire::digest("accountability-event", event)?
            || ack["payload_digest"] != event["payload_digest"]
            || ack["event_id"] != event["event_id"]
        {
            return Err(Error::Refused);
        }
        let raw = wire::canonical(ack)?;
        let changed = self.db.execute(
            "UPDATE outbox SET ack=?3 WHERE scope=?1 AND digest=?2 AND (ack IS NULL OR ack=?3)",
            params![
                wire::canonical(scope)?,
                wire::digest("accountability-event", event)?,
                raw
            ],
        )?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }
}

fn view(record: Value, status: &str, revision: u64, auth: &Authority) -> Value {
    let usable = status == "approved"
        && record["approver"] == auth.approver
        && record["eligibility_revision"] == auth.eligibility_revision
        && record["context_digest"] == wire::digest("context", &auth.context).unwrap_or_default()
        && record["issued_at"]
            .as_u64()
            .is_some_and(|n| n.saturating_add(2) <= auth.now)
        && record["expires_at"]
            .as_u64()
            .is_some_and(|n| auth.now.saturating_add(2) < n);
    json!({"approval":record,"status":status,"revision":revision,"currently_usable":usable})
}

fn validate_inputs(auth: &Authority, request: &Value, decision: &Value) -> Result<()> {
    wire::shape(request, "action-request")?;
    wire::shape(decision, "action-decision")?;
    wire::scope(request, &auth.scope)?;
    wire::scope(decision, &auth.scope)?;
    wire::scope(&auth.approver, &auth.scope)?;
    if auth.approver["kind"] != "human"
        || auth.approver == request["intent"]["actor"]
        || auth.approver == request["intent"]["origin"]
        || auth.requester_chain.contains(&auth.approver)
        || !auth.requester_chain.contains(&request["intent"]["actor"])
        || !auth.requester_chain.contains(&request["intent"]["origin"])
        || auth.eligibility_revision == 0
        || auth.generation == 0
        || auth.context != request["context"]
    {
        return Err(Error::Refused);
    }
    if request["intent_digest"] != wire::digest("intent", &request["intent"])?
        || request["context_digest"] != wire::digest("context", &request["context"])?
        || decision["request_digest"] != wire::digest("action-request", request)?
        || decision["context_digest"] != request["context_digest"]
        || decision["operation"] != request["operation"]
        || decision["attempt"] != request["attempt"]
        || decision["outcome"] != "approval-required"
    {
        return Err(Error::Refused);
    }
    if auth.now < wire::number(&auth.context["valid_from"])?.saturating_add(2)
        || auth.now.saturating_add(2) >= wire::number(&auth.context["expires_at"])?
    {
        return Err(Error::Refused);
    }
    let obligations = decision["obligations"].as_array().ok_or(Error::Invalid)?;
    let expected = [
        "distinct-approval",
        "mandatory-recording",
        "action-capacity",
        "credential-custody",
    ];
    if obligations.len() != 4
        || obligations
            .iter()
            .zip(expected)
            .any(|(o, k)| o["kind"] != k)
        || obligations[0]["policy_digest"] != auth.context["policy_digest"]
        || obligations[2]["target"] != request["intent"]["target"]
        || obligations[3]["audience"] != request["intent"]["target"]
    {
        return Err(Error::Refused);
    }
    Ok(())
}
