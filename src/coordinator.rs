// SPDX-License-Identifier: Apache-2.0
//! Durable governance ratification and activation progress. This owns no participant tables.
use crate::wire::{self, Error, Result};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::path::Path;
const PARTICIPANTS: [&str; 4] = ["gate", "registry", "server", "warden"];
/// Independently admitted current ratifier. A request cannot supply this context.
pub struct Ratifier {
    /// Qualified deployment scope.
    pub scope: Value,
    /// Enrolled human principal.
    pub principal: Value,
    /// Current enrollment revision.
    pub revision: u64,
    /// Current bounded UTC time.
    pub now: u64,
}
/// Restartable activation coordinator with append-only transitions and progress history.
pub struct Coordinator {
    db: Connection,
}
impl Coordinator {
    /// Open additive operational and durable delivery tables.
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
          CREATE TABLE IF NOT EXISTS transitions(scope TEXT NOT NULL,id TEXT NOT NULL,record TEXT NOT NULL,digest TEXT NOT NULL,proposer TEXT NOT NULL,ratifier TEXT,eligibility INTEGER,pause TEXT,resumed INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(scope,id));
          CREATE TABLE IF NOT EXISTS transition_receipts(scope TEXT NOT NULL,id TEXT NOT NULL,participant TEXT NOT NULL,receipt TEXT NOT NULL,PRIMARY KEY(scope,id,participant));
          CREATE TABLE IF NOT EXISTS transition_history(seq INTEGER PRIMARY KEY,scope TEXT NOT NULL,id TEXT NOT NULL,kind TEXT NOT NULL,record TEXT NOT NULL);")?;
        Ok(Self { db })
    }
    /// Retain an inert transition proposed by an independently authenticated principal.
    pub fn propose(
        &mut self,
        scope: &Value,
        proposer: &Value,
        transition: &Value,
        now: u64,
    ) -> Result<Value> {
        validate(scope, transition, now)?;
        wire::scope(proposer, scope)?;
        if proposer["subject"].as_str().is_none()
            || !["human", "agent", "service"]
                .iter()
                .any(|k| proposer["kind"] == *k)
        {
            return Err(Error::Refused);
        }
        let key = wire::canonical(scope)?;
        let id = transition["transition"]["id"]
            .as_str()
            .ok_or(Error::Invalid)?;
        let raw = wire::canonical(transition)?;
        let actor = wire::canonical(proposer)?;
        let digest = wire::digest("activation", transition)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old: Option<(String, String)> = tx
            .query_row(
                "SELECT digest,proposer FROM transitions WHERE scope=?1 AND id=?2",
                params![key, id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, previous)) = old {
            if hash != digest || previous != actor {
                return Err(Error::Conflict);
            }
        } else {
            tx.execute(
                "INSERT INTO transitions(scope,id,record,digest,proposer) VALUES(?1,?2,?3,?4,?5)",
                params![key, id, raw, digest, actor],
            )?;
            tx.execute(
                "INSERT INTO transition_history(scope,id,kind,record) VALUES(?1,?2,'proposed',?3)",
                params![key, id, raw],
            )?;
        }
        tx.commit()?;
        self.lookup(scope, id)
    }
    /// Ratify exact immutable bytes with an eligible human distinct from the proposer.
    pub fn ratify(&mut self, auth: &Ratifier, id: &str) -> Result<Value> {
        let key = wire::canonical(&auth.scope)?;
        let actor = wire::canonical(&auth.principal)?;
        if auth.principal["kind"] != "human" || auth.revision == 0 {
            return Err(Error::Refused);
        }
        wire::scope(&auth.principal, &auth.scope)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (raw,proposer,old,revision):(String,String,Option<String>,Option<u64>)=tx.query_row("SELECT record,proposer,ratifier,eligibility FROM transitions WHERE scope=?1 AND id=?2",params![key,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(Error::Refused)?;
        validate(&auth.scope, &wire::parse(&raw)?, auth.now)?;
        if proposer == actor {
            return Err(Error::Refused);
        }
        if let Some(old) = old {
            if old != actor || revision != Some(auth.revision) {
                return Err(Error::Conflict);
            }
        } else {
            tx.execute(
                "UPDATE transitions SET ratifier=?3,eligibility=?4 WHERE scope=?1 AND id=?2",
                params![key, id, actor, auth.revision],
            )?;
            tx.execute(
                "INSERT INTO transition_history(scope,id,kind,record) VALUES(?1,?2,'ratified',?3)",
                params![key, id, actor],
            )?;
        }
        tx.commit()?;
        self.lookup(&auth.scope, id)
    }
    /// Expose each participant separately; ratification is not global activation.
    pub fn lookup(&self, scope: &Value, id: &str) -> Result<Value> {
        let key = wire::canonical(scope)?;
        let (raw,digest,ratifier,eligibility,pause,resumed):(String,String,Option<String>,Option<u64>,Option<String>,bool)=self.db.query_row("SELECT record,digest,ratifier,eligibility,pause,resumed FROM transitions WHERE scope=?1 AND id=?2",params![key,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(Error::Refused)?;
        let mut stmt = self.db.prepare(
            "SELECT receipt FROM transition_receipts WHERE scope=?1 AND id=?2 ORDER BY participant",
        )?;
        let receipts: Vec<Value> = stmt
            .query_map(params![key, id], |r| r.get::<_, String>(0))?
            .map(|r| wire::parse(&r?))
            .collect::<Result<_>>()?;
        Ok(
            json!({"transition":wire::parse(&raw)?,"transition_digest":digest,"ratified":ratifier.is_some(),"ratifier":ratifier.map(|s|wire::parse(&s)).transpose()?,"eligibility_revision":eligibility,"pause":pause.map(|s|wire::parse(&s)).transpose()?,"receipts":receipts,"resumed":resumed}),
        )
    }
    /// Retain a receipt obtained directly from the named authenticated participant.
    pub fn receipt(
        &mut self,
        scope: &Value,
        id: &str,
        participant: &str,
        receipt: &Value,
    ) -> Result<()> {
        let key = wire::canonical(scope)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (raw, ratifier, pause): (String, Option<String>, Option<String>) = tx
            .query_row(
                "SELECT record,ratifier,pause FROM transitions WHERE scope=?1 AND id=?2",
                params![key, id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or(Error::Refused)?;
        if ratifier.is_none() {
            return Err(Error::Refused);
        }
        let transition = wire::parse(&raw)?;
        check_receipt(scope, &transition, participant, receipt)?;
        let encoded = wire::canonical(receipt)?;
        if receipt["phase"] == "paused" {
            if participant != "gate" || pause.as_ref().is_some_and(|p| p != &encoded) {
                return Err(Error::Conflict);
            }
            if pause.is_none() {
                tx.execute(
                    "UPDATE transitions SET pause=?3 WHERE scope=?1 AND id=?2",
                    params![key, id, encoded],
                )?;
                tx.execute("INSERT INTO transition_history(scope,id,kind,record) VALUES(?1,?2,'paused',?3)",params![key,id,encoded])?;
            }
        } else {
            if pause.is_none() {
                return Err(Error::Refused);
            }
            let old:Option<String>=tx.query_row("SELECT receipt FROM transition_receipts WHERE scope=?1 AND id=?2 AND participant=?3",params![key,id,participant],|r|r.get(0)).optional()?;
            if old.as_ref().is_some_and(|p| p != &encoded) {
                return Err(Error::Conflict);
            }
            if old.is_none() {
                tx.execute(
                    "INSERT INTO transition_receipts VALUES(?1,?2,?3,?4)",
                    params![key, id, participant, encoded],
                )?;
                tx.execute("INSERT INTO transition_history(scope,id,kind,record) VALUES(?1,?2,'applied',?3)",params![key,id,encoded])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    /// Prepare a resume command only after all four participant receipts are retained.
    pub fn completion(&self, scope: &Value, id: &str) -> Result<Value> {
        let state = self.lookup(scope, id)?;
        if state["pause"].is_null() || state["receipts"].as_array().is_none_or(|r| r.len() != 4) {
            return Err(Error::Refused);
        }
        Ok(
            json!({"transition":state["transition"],"pause":state["pause"],"receipts":state["receipts"]}),
        )
    }
    /// Record a successful authenticated Gate resume reply; failed replies remain pending.
    pub fn resumed(&mut self, scope: &Value, id: &str, reply: &Value) -> Result<()> {
        self.completion(scope, id)?;
        let state = self.lookup(scope, id)?;
        if reply["transition_digest"] != state["transition_digest"] || reply["resumed"] != true {
            return Err(Error::Refused);
        }
        let key = wire::canonical(scope)?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE transitions SET resumed=1 WHERE scope=?1 AND id=?2 AND resumed=0",
            params![key, id],
        )? == 1
        {
            tx.execute(
                "INSERT INTO transition_history(scope,id,kind,record) VALUES(?1,?2,'resumed',?3)",
                params![key, id, wire::canonical(reply)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
fn validate(scope: &Value, t: &Value, now: u64) -> Result<()> {
    wire::shape(t, "activation")?;
    wire::scope(t, scope)?;
    let profile: Value = serde_json::from_str(include_str!("../contracts/stage2-v1/profile.json"))
        .map_err(|_| Error::Unavailable)?;
    if t["participants"] != json!(PARTICIPANTS)
        || t["profile_digest"] != wire::digest("profile", &profile)?
        || t["artifact_set_digest"]
            != wire::digest("artifact-set", &json!({"artifacts":t["artifacts"]}))?
        || t["participant_set_digest"]
            != wire::digest("participants", &json!({"participants":PARTICIPANTS}))?
        || wire::number(&t["successor_epoch"])? <= wire::number(&t["prior_epoch"])?
    {
        return Err(Error::Refused);
    }
    if now < wire::number(&t["not_before"])?.saturating_add(2)
        || now.saturating_add(2) >= wire::number(&t["expires_at"])?
    {
        return Err(Error::Refused);
    }
    Ok(())
}
fn check_receipt(scope: &Value, t: &Value, participant: &str, r: &Value) -> Result<()> {
    wire::shape(r, "activation-receipt")?;
    wire::scope(r, scope)?;
    if !PARTICIPANTS.contains(&participant)
        || r["participant"] != participant
        || r["transition_digest"] != wire::digest("activation", t)?
    {
        return Err(Error::Refused);
    }
    for key in [
        "transition",
        "prior_epoch",
        "successor_epoch",
        "artifact_set_digest",
        "participant_set_digest",
    ] {
        if r[key] != t[key] {
            return Err(Error::Refused);
        }
    }
    Ok(())
}
