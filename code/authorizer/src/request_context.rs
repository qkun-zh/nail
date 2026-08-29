/// Request metadata the application vouches for and exposes to Cedar as the
/// request context.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestContext {
    /// Whether the request is the email-confirmed account-deregistration
    /// flow. Only honored for `User::Delete::Soft`; other actions see an
    /// empty context.
    pub delete_token_confirmed: bool,
}

impl RequestContext {
    /// The action whose schema declares the `delete_token_confirmed` context
    /// attribute.
    pub const DELETE_TOKEN_CONFIRMED_ACTION: &str = "User::Delete::Soft";
}
