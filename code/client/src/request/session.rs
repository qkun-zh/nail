use std::sync::OnceLock;

use crate::infrastructure::storage;

pub const SESSION_TOKEN_KEY: &str = "session_token";

pub fn read_session_token() -> Option<String> {
    storage::read(SESSION_TOKEN_KEY)
}

pub fn store_session_token(token: &str) {
    storage::write(SESSION_TOKEN_KEY, token);
}

pub fn clear_session_token() {
    storage::remove(SESSION_TOKEN_KEY);
}

pub fn should_invalidate_session(status: u16, authenticated: bool) -> bool {
    authenticated && status == 401
}

static SESSION_INVALID_HOOK: OnceLock<fn()> = OnceLock::new();

pub fn set_session_invalid_hook(hook: fn()) {
    let _ = SESSION_INVALID_HOOK.set(hook);
}

pub fn notify_session_invalid() {
    if let Some(hook) = SESSION_INVALID_HOOK.get() {
        hook();
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
