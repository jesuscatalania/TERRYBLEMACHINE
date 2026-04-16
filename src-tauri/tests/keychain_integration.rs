//! Real macOS Keychain round-trip. Ignored by default because it writes to the
//! login keychain and can trigger access prompts.
//!
//! Run manually with:
//!
//! ```sh
//! cargo test --test keychain_integration -- --ignored
//! ```

#![cfg(target_os = "macos")]

use terryblemachine_lib::keychain::{KeyStore, KeychainStore};

#[test]
#[ignore]
fn keychain_round_trip_stores_reads_deletes() {
    let store = KeychainStore::new("com.terryblemachine.test");
    let service = "test_service";

    store.store(service, "test123").expect("store");

    let value = store.get(service).expect("get");
    assert_eq!(value, "test123");

    store.delete(service).expect("delete");

    let err = store.get(service).expect_err("get after delete");
    match err {
        terryblemachine_lib::keychain::KeyStoreError::NotFound(s) => assert_eq!(s, service),
        other => panic!("expected NotFound, got {other:?}"),
    }
}
