pub fn validate_name(raw: &str) -> Result<String, String> {
    common::name::validate_name(raw).map_err(|error| error.to_string())
}

pub fn validate_title(raw: &str, max_chars: u64) -> Result<String, String> {
    common::text::validate_ascii_text(raw, usize::try_from(max_chars).unwrap_or(usize::MAX), false)
        .map_err(|error| error.to_string())
}

pub fn validate_summary(raw: &str, max_chars: u64) -> Result<String, String> {
    common::text::validate_ascii_text(raw, usize::try_from(max_chars).unwrap_or(usize::MAX), true)
        .map_err(|error| error.to_string())
}

pub fn validate_note(raw: &str, max_chars: u64) -> Result<String, String> {
    common::text::validate_ascii_text(raw, usize::try_from(max_chars).unwrap_or(usize::MAX), true)
        .map_err(|error| error.to_string())
}

pub fn validate_comment_content(raw: &str, max_chars: u64) -> Result<String, String> {
    common::text::validate_ascii_text(raw, usize::try_from(max_chars).unwrap_or(usize::MAX), true)
        .map_err(|error| error.to_string())
}

pub fn validate_uuid(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    let bytes = value.as_bytes();
    let valid = value.len() == 36
        && bytes.get(8) == Some(&b'-')
        && bytes.get(13) == Some(&b'-')
        && bytes.get(18) == Some(&b'-')
        && bytes.get(23) == Some(&b'-')
        && is_uuid_hex(&value[0..8])
        && is_uuid_hex(&value[9..13])
        && is_uuid_hex(&value[14..18])
        && is_uuid_hex(&value[19..23])
        && is_uuid_hex(&value[24..36]);
    if valid {
        Ok(value.to_string())
    } else {
        Err("invalid id: expected a UUID".to_string())
    }
}

fn is_uuid_hex(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn looks_like_pdf(mime_type: &str, file_name: &str) -> bool {
    matches!(
        mime_type,
        "" | "application/pdf" | "application/octet-stream"
    ) || file_name.to_lowercase().ends_with(".pdf")
}

pub fn validate_pdf_selection(
    mime_type: &str,
    file_name: &str,
    size: u64,
    max_size: u64,
) -> Result<(), String> {
    if !looks_like_pdf(mime_type, file_name) {
        return Err("only PDF files are allowed".to_string());
    }
    if size > max_size {
        return Err(format!("file too large: {size} > {max_size} bytes"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
