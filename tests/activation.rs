// SPDX-License-Identifier: Apache-2.0
use munarium_council::{
    coordinator::{Coordinator, Ratifier},
    wire,
};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../contracts/stage2-v1/vectors.json")).unwrap()
}
#[test]
fn partial_activation_survives_every_receipt_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("c.sqlite");
    let v = vectors();
    let r = &v["records"];
    let scope = &v["trusted"]["scope"];
    let t = &r["activation"];
    let mut c = Coordinator::open(&p).unwrap();
    c.propose(scope, &v["trusted"]["actor"], t, 1000).unwrap();
    assert_eq!(c.lookup(scope, "transition-a").unwrap()["ratified"], false);
    assert!(
        c.receipt(scope, "transition-a", "gate", &r["pause"])
            .is_err()
    );
    let a = Ratifier {
        scope: scope.clone(),
        principal: v["trusted"]["approver"].clone(),
        revision: 1,
        now: 1000,
    };
    c.ratify(&a, "transition-a").unwrap();
    c.receipt(scope, "transition-a", "gate", &r["pause"])
        .unwrap();
    for participant in ["registry", "server", "warden", "gate"] {
        assert!(c.completion(scope, "transition-a").is_err());
        c.receipt(
            scope,
            "transition-a",
            participant,
            &r[format!("{participant}-receipt")],
        )
        .unwrap();
        drop(c);
        c = Coordinator::open(&p).unwrap();
        assert_eq!(c.lookup(scope, "transition-a").unwrap()["resumed"], false);
    }
    c.completion(scope, "transition-a").unwrap();
    assert!(
        c.resumed(
            scope,
            "transition-a",
            &json!({"resumed":true,"transition_digest":"wrong"})
        )
        .is_err()
    );
    let reply = json!({"resumed":true,"transition_digest":wire::digest("activation",t).unwrap()});
    c.resumed(scope, "transition-a", &reply).unwrap();
    drop(c);
    assert_eq!(
        Coordinator::open(&p)
            .unwrap()
            .lookup(scope, "transition-a")
            .unwrap()["resumed"],
        true
    );
}
#[test]
fn proposer_cannot_ratify_and_changed_receipts_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let v = vectors();
    let r = &v["records"];
    let scope = &v["trusted"]["scope"];
    let mut c = Coordinator::open(&dir.path().join("c.sqlite")).unwrap();
    let principal = v["trusted"]["approver"].clone();
    c.propose(scope, &principal, &r["activation"], 1000)
        .unwrap();
    let mut a = Ratifier {
        scope: scope.clone(),
        principal: principal.clone(),
        revision: 1,
        now: 1000,
    };
    assert!(c.ratify(&a, "transition-a").is_err());
    a.principal["subject"] = json!("other-human");
    c.ratify(&a, "transition-a").unwrap();
    assert!(
        c.receipt(scope, "transition-a", "registry", &r["registry-receipt"])
            .is_err()
    );
    c.receipt(scope, "transition-a", "gate", &r["pause"])
        .unwrap();
    let mut wrong = r["registry-receipt"].clone();
    wrong["successor_epoch"] = json!(3);
    assert!(
        c.receipt(scope, "transition-a", "registry", &wrong)
            .is_err()
    );
    assert!(
        c.receipt(scope, "transition-a", "warden", &r["registry-receipt"])
            .is_err()
    );
    let mut foreign = scope.clone();
    foreign["tenant"] = json!("foreign");
    assert!(c.lookup(&foreign, "transition-a").is_err());
}
