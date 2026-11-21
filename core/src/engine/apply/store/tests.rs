//! Store tests.

use super::*;
use crate::engine::testutil::tmpdir;

#[test]
fn json_writes_are_atomic_and_valid() {
    // write_json must leave a valid file and never leave .tmp leftovers
    let dir = tmpdir("atomic");
    let store = Store::new(&dir);
    let st = State {
        active: Some("game".into()),
        updated: 7,
        last_mode: "apply".into(),
    };
    store.save_state(&st).unwrap();
    assert_eq!(store.load_state().active.as_deref(), Some("game"));
    // no stray tmp files in the state dir
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "tmp").unwrap_or(false))
        .collect();
    assert!(leftovers.is_empty(), "tmp leftovers: {leftovers:?}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn state_roundtrip() {
    let dir = tmpdir("state");
    let store = Store::new(&dir);
    assert!(store.load_state().active.is_none());
    store
        .save_state(&State {
            active: Some("game".into()),
            updated: 42,
            last_mode: "apply".into(),
        })
        .unwrap();
    let st = store.load_state();
    assert_eq!(st.active.as_deref(), Some("game"));
    assert_eq!(st.updated, 42);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn embedded_profiles_load_without_file() {
    let dir = tmpdir("profiles");
    let store = Store::new(&dir);
    let p = store.load_profiles(None).unwrap();
    assert_eq!(p.profiles.len(), 4); // powersave, balance, game, sleep
    let _ = fs::remove_dir_all(&dir);
}
