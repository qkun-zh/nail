/// Decides whether a single character is accepted by a validation policy.
pub trait CharPolicy {
    fn allows(&self, ch: char) -> bool;
}

/// Builds the domain error for each shared validation failure kind.
pub trait ValidationError: Sized {
    fn empty() -> Self;
    fn too_long(max_chars: usize) -> Self;
    fn forbidden(ch: char) -> Self;
}

/// Shared skeleton: trim, reject blank, reject forbidden chars, enforce a char
/// count cap, then return the trimmed string.
///
/// # Errors
/// Returns an `E::empty()` error for blank input, `E::too_long(max_chars)` when
/// the trimmed char count exceeds `max_chars`, or `E::forbidden(ch)` for the
/// first character the policy rejects.
pub fn validate_with_policy<E: ValidationError, P: CharPolicy>(
    raw: &str,
    max_chars: usize,
    policy: &P,
) -> Result<String, E> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(E::empty());
    }
    for ch in trimmed.chars() {
        if !policy.allows(ch) {
            return Err(E::forbidden(ch));
        }
    }
    if trimmed.chars().count() > max_chars {
        return Err(E::too_long(max_chars));
    }
    Ok(trimmed.to_string())
}

/// Accepts ASCII alphanumerics plus `-` and `_`.
pub struct AlphanumericDashUnderscore;

impl CharPolicy for AlphanumericDashUnderscore {
    fn allows(&self, ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
    }
}

/// Accepts printable ASCII 0x20..=0x7e, plus `\n` when `allow_newline` is set.
pub struct PrintableAscii {
    pub allow_newline: bool,
}

impl CharPolicy for PrintableAscii {
    fn allows(&self, ch: char) -> bool {
        ch.is_ascii() && ((0x20..=0x7e).contains(&(ch as u8)) || (self.allow_newline && ch == '\n'))
    }
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
