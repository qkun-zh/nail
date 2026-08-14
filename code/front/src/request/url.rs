const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

pub fn encode_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if is_encode_uri_component_safe(*byte) {
            output.push(*byte as char);
        } else {
            output.push('%');
            output.push(HEX_UPPER[(byte >> 4) as usize] as char);
            output.push(HEX_UPPER[(byte & 0x0f) as usize] as char);
        }
    }
    output
}

fn is_encode_uri_component_safe(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')')
}

pub fn build_path_with_query(path_segments: &[&str], query: &[(&str, &str)]) -> String {
    let mut path = String::from("/");
    path.push_str(
        &path_segments
            .iter()
            .map(|segment| encode_component(segment))
            .collect::<Vec<_>>()
            .join("/"),
    );
    if !query.is_empty() {
        let query_string = query
            .iter()
            .map(|(key, value)| format!("{}={}", encode_component(key), encode_component(value)))
            .collect::<Vec<_>>()
            .join("&");
        path.push('?');
        path.push_str(&query_string);
    }
    path
}

#[cfg(test)]
#[path = "../../../../test/unit/front/request/url/tests.rs"]
mod tests;
