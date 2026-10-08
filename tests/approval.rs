// SPDX-License-Identifier: Apache-2.0
use munarium_council::{
    store::{Authority, Store},
    wire::{self, Error},
};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../contracts/stage2-v1/vectors.json")).unwrap()
}
fn authority(v: &Value) -> Authority {
    let t = &v["trusted"];
    Authority {
        scope: t["scope"].clone(),
        approver: t["approver"].clone(),
        eligibility_revision: 1,
        requester_chain: vec![v["records"]["request"]["intent"]["actor"].clone()],
        context: v["records"]["request"]["context"].clone(),
        stream: "council-events".into(),
        generation: 1,
        now: 1000,
    }
}
#[test]
fn canonical_vectors_are_independently_consumed() {
    let v = vectors();
    for (name, record) in v["records"].as_object().unwrap() {
        assert_eq!(
            wire::canonical(record).unwrap(),
            v["canonical"][name].as_str().unwrap()
        );
        assert_eq!(
            wire::digest(record["type"].as_str().unwrap(), record).unwrap(),
            v["digests"][name].as_str().unwrap()
        );
        wire::shape(record, record["type"].as_str().unwrap()).unwrap();
    }
    for raw in [
        "{\"a\":1,\"a\":1}",
        "{\"a\":1.0}",
        "{ \"a\":1}",
        "{\"a\":9007199254740992}",
    ] {
        assert_eq!(wire::parse(raw), Err(Error::Invalid));
    }
}
#[test]
fn approval_retry_restart_outbox_and_current_status() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("council.sqlite");
    let v = vectors();
    let r = &v["records"];
    let mut a = authority(&v);
    let mut store = Store::open(&path).unwrap();
    let first = store
        .approve(&a, &r["request"], &r["decision"], "approval-a")
        .unwrap();
    assert_eq!(first["currently_usable"], false);
    assert_eq!(store.pending(&a.scope).unwrap().len(), 1);
    drop(store);
    let mut store = Store::open(&path).unwrap();
    a.now += 3;
    let retry = store
        .approve(&a, &r["request"], &r["decision"], "approval-a")
        .unwrap();
    assert_eq!(first["approval"], retry["approval"]);
    assert_eq!(retry["currently_usable"], true);
    assert_eq!(store.pending(&a.scope).unwrap().len(), 1);
    assert_eq!(
        store.approve(&a, &r["request"], &r["decision"], "approval-b"),
        Err(Error::Conflict)
    );
    a.eligibility_revision = 2;
    assert_eq!(
        store.lookup(&a, "approval-a").unwrap()["currently_usable"],
        false
    );
    assert_eq!(
        store.approve(&a, &r["request"], &r["decision"], "approval-a"),
        Err(Error::Conflict)
    );
    a.eligibility_revision = 1;
    a.now = 1299;
    assert_eq!(
        store.lookup(&a, "approval-a").unwrap()["currently_usable"],
        false
    );
    a.scope["tenant"] = json!("tenant-b");
    assert_eq!(store.lookup(&a, "approval-a"), Err(Error::Refused));
}
#[test]
fn substitutions_self_approval_and_stale_context_refuse_without_writes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("c.sqlite")).unwrap();
    let v = vectors();
    let r = &v["records"];
    let mut a = authority(&v);
    a.approver = r["request"]["intent"]["actor"].clone();
    assert!(
        store
            .approve(&a, &r["request"], &r["decision"], "x")
            .is_err()
    );
    let mut a = authority(&v);
    a.requester_chain.push(a.approver.clone());
    assert!(
        store
            .approve(&a, &r["request"], &r["decision"], "x")
            .is_err()
    );
    let mut a = authority(&v);
    a.context["activation"]["revision"] = json!(3);
    assert!(
        store
            .approve(&a, &r["request"], &r["decision"], "x")
            .is_err()
    );
    let a = authority(&v);
    for field in ["request_digest", "context_digest"] {
        let mut decision = r["decision"].clone();
        decision[field] = json!(format!("sha256:{}", "0".repeat(64)));
        assert!(store.approve(&a, &r["request"], &decision, "x").is_err());
    }
    let mut request = r["request"].clone();
    request["intent"]["target"]["id"] = json!("another-target");
    assert!(store.approve(&a, &request, &r["decision"], "x").is_err());
    assert!(store.pending(&a.scope).unwrap().is_empty());
}
#[test]
fn withdrawal_requires_exact_durable_gate_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.sqlite");
    let v = vectors();
    let r = &v["records"];
    let a = authority(&v);
    let mut store = Store::open(&path).unwrap();
    let first = store
        .approve(&a, &r["request"], &r["decision"], "approval-a")
        .unwrap();
    let pending = store
        .withdraw(&a.scope, "approval-a", "withdraw-a")
        .unwrap();
    assert_eq!(pending["status"], "withdrawal-pending");
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .withdraw(&a.scope, "approval-a", "withdraw-a")
            .unwrap(),
        pending
    );
    assert_eq!(
        store.withdraw(&a.scope, "approval-a", "changed"),
        Err(Error::Conflict)
    );
    let mut receipt = json!({"approval":first["approval"]["approval"],"approval_revision":1,"operation":r["request"]["operation"],"attempt":r["request"]["attempt"],"withdrawal_id":"wrong","status":"cancelled"});
    assert_eq!(
        store.finish_withdrawal(&a.scope, "approval-a", &receipt),
        Err(Error::Refused)
    );
    receipt["withdrawal_id"] = json!("withdraw-a");
    receipt["approval_revision"] = json!(2);
    assert_eq!(
        store.finish_withdrawal(&a.scope, "approval-a", &receipt),
        Err(Error::Conflict)
    );
    receipt["approval_revision"] = json!(1);
    store
        .finish_withdrawal(&a.scope, "approval-a", &receipt)
        .unwrap();
    store
        .finish_withdrawal(&a.scope, "approval-a", &receipt)
        .unwrap();
    assert_eq!(
        store.lookup(&a, "approval-a").unwrap()["status"],
        "withdrawn"
    );
    assert_eq!(
        store.lookup(&a, "approval-a").unwrap()["approval"],
        first["approval"]
    );
}
#[test]
fn concurrent_approval_has_one_durable_issuance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.sqlite");
    drop(Store::open(&path).unwrap());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let p = path.clone();
            std::thread::spawn(move || {
                let v = vectors();
                Store::open(&p)
                    .unwrap()
                    .approve(
                        &authority(&v),
                        &v["records"]["request"],
                        &v["records"]["decision"],
                        "approval-a",
                    )
                    .unwrap()
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.iter().all(|r| r == &results[0]));
    assert_eq!(
        Store::open(&path)
            .unwrap()
            .pending(&authority(&vectors()).scope)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn exact_audit_ack_is_required_before_delivery_is_complete() {
    let dir = tempfile::tempdir().unwrap();
    let v = vectors();
    let a = authority(&v);
    let mut store = Store::open(&dir.path().join("c.sqlite")).unwrap();
    store
        .approve(
            &a,
            &v["records"]["request"],
            &v["records"]["decision"],
            "approval-a",
        )
        .unwrap();
    let event = store.pending(&a.scope).unwrap().remove(0);
    let mut ack = v["records"]["ack"].clone();
    assert_eq!(
        store.acknowledge(&a.scope, &event, &ack),
        Err(Error::Refused)
    );
    assert_eq!(store.pending(&a.scope).unwrap().len(), 1);
    ack["event_id"] = event["event_id"].clone();
    ack["payload_digest"] = event["payload_digest"].clone();
    ack["event_digest"] = json!(wire::digest("accountability-event", &event).unwrap());
    store.acknowledge(&a.scope, &event, &ack).unwrap();
    store.acknowledge(&a.scope, &event, &ack).unwrap();
    assert!(store.pending(&a.scope).unwrap().is_empty());
    ack["position"] = json!(2);
    assert_eq!(
        store.acknowledge(&a.scope, &event, &ack),
        Err(Error::Conflict)
    );
}
