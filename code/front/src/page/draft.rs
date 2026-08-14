use crate::request::url::encode_component;

pub fn build_draft_query(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{}={}", encode_component(key), encode_component(value)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn draft_url(pathname: &str, fields: &[(&str, &str)]) -> String {
    let query = build_draft_query(fields);
    if query.is_empty() {
        pathname.to_string()
    } else {
        format!("{pathname}?{query}")
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/draft/tests.rs"]
mod tests;
