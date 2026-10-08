// SPDX-License-Identifier: Apache-2.0
use serde_json::Value;
use sha2::{Digest, Sha256};
#[test]
fn candidate_and_transport_exports_match_their_locks() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (folder, lock) in [
        ("contracts/stage2-v1", "vendor-lock.json"),
        ("vendor/warden-transport", "source-lock.json"),
    ] {
        let path = root.join(folder);
        let value: Value =
            serde_json::from_slice(&std::fs::read(path.join(lock)).unwrap()).unwrap();
        for (file, expected) in value["files"].as_object().unwrap() {
            let raw = std::fs::read(path.join(file)).unwrap();
            let text = String::from_utf8(raw).unwrap().replace("\r\n", "\n");
            assert_eq!(
                format!("{:x}", Sha256::digest(text.as_bytes())),
                expected.as_str().unwrap(),
                "{folder}/{file}"
            );
        }
    }
}
