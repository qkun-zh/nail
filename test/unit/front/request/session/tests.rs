use std::sync::Mutex;

use crate::request::session::{
    SESSION_TOKEN_KEY, notify_session_invalid, set_session_invalid_hook, should_invalidate_session,
};

#[test]
fn invalidates_only_an_authenticated_401() {
    assert!(should_invalidate_session(401, true));
    assert!(!should_invalidate_session(401, false));
    assert!(!should_invalidate_session(403, true));
    assert!(!should_invalidate_session(400, true));
    assert!(!should_invalidate_session(200, true));
    assert!(!should_invalidate_session(500, true));
}

#[test]
fn session_token_key_matches_the_backend_contract() {
    assert_eq!(SESSION_TOKEN_KEY, "session_token");
}

#[test]
fn invalidation_hook_fires_only_once_registered() {
    static CALLS: Mutex<u32> = Mutex::new(0);
    fn mark() {
        *CALLS.lock().expect("lock") += 1;
    }
    set_session_invalid_hook(mark);
    notify_session_invalid();
    notify_session_invalid();
    assert_eq!(*CALLS.lock().expect("lock"), 2);
}
