pub fn validate_name(raw: &str) -> Result<String, String> {
    nail_common::name::validate_name(raw).map_err(|error| error.to_string())
}

pub fn validate_title(raw: &str, max_chars: u64) -> Result<String, String> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        false,
    )
    .map_err(|error| error.to_string())
}

pub fn validate_summary(raw: &str, max_chars: u64) -> Result<String, String> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| error.to_string())
}

pub fn validate_note(raw: &str, max_chars: u64) -> Result<String, String> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| error.to_string())
}

pub fn validate_comment_content(raw: &str, max_chars: u64) -> Result<String, String> {
    nail_common::text::validate_ascii_text(
        raw,
        usize::try_from(max_chars).unwrap_or(usize::MAX),
        true,
    )
    .map_err(|error| error.to_string())
}

pub fn validate_tags(raw: &str, max_count: usize) -> Result<Vec<String>, String> {
    let tags = nail_common::tag::parse_tags(raw, max_count).map_err(|error| error.to_string())?;
    if tags.is_empty() {
        return Err("at least one tag is required".to_string());
    }
    Ok(tags)
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
#[path = "../../../../test/unit/front/page/validation/tests.rs"]
mod tests;
