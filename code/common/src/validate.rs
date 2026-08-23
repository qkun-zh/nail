pub trait CharPolicy {
    fn allows(&self, ch: char) -> bool;
}

pub trait ValidationError: Sized {
    fn empty() -> Self;
    fn too_long(max_chars: usize) -> Self;
    fn forbidden(ch: char) -> Self;
}

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

pub struct AlphanumericDashUnderscore;

impl CharPolicy for AlphanumericDashUnderscore {
    fn allows(&self, ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
    }
}

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
