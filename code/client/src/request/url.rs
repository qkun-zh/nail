use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

const COMPONENT_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

pub fn encode_component(value: &str) -> String {
    utf8_percent_encode(value, COMPONENT_SET).to_string()
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
#[path = "url_tests.rs"]
mod tests;
