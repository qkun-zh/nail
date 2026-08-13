
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("cwd");
    cwd.ancestors()
        .find(|dir| dir.join("conf/back/server.toml").is_file())
        .expect("cannot locate repo root (conf/back/server.toml) from cwd")
        .to_path_buf()
}

fn fn_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("fn {name}() -> ");
    let start = src.find(&marker)? + marker.len();
    let brace = src[start..].find('{')? + start + 1;
    let body_start = brace;
    let close = src[brace..].find('}')? + brace;
    Some(src[body_start..close].to_string())
}

fn extract_int_literal(body: &str) -> Option<u64> {
    let cleaned: String = body
        .chars()
        .filter(|c| c.is_ascii_digit() || c.is_whitespace() || *c == '*')
        .collect();
    let expr = cleaned.split_whitespace().collect::<String>();
    if expr.is_empty() {
        return None;
    }
    let terms: Vec<&str> = expr.split('*').collect();
    if terms.is_empty() {
        return None;
    }
    let mut product: u64 = 1;
    for term in terms {
        let term = term.trim();
        if term.is_empty() {
            return None;
        }
        product *= term.parse::<u64>().ok()?;
    }
    Some(product)
}

fn load_server_toml(root: &Path) -> toml::Table {
    let text = std::fs::read_to_string(root.join("conf/back/server.toml"))
        .expect("read conf/back/server.toml");
    text.parse::<toml::Table>().expect("parse server.toml")
}

fn conf_u64(table: &toml::Table, key: &str) -> u64 {
    table
        .get(key)
        .and_then(|v| v.as_integer())
        .expect("conf key")
        .try_into()
        .expect("conf value fits u64")
}

#[test]
fn front_fallback_defaults_match_server_configuration() {
    let root = repo_root();
    let limits_src = std::fs::read_to_string(root.join("code/front/src/limits.rs"))
        .expect("read code/front/src/limits.rs");
    let conf = load_server_toml(&root);

    let checks = [
        ("default_max_tags", "max_tags_per_article"),
        ("default_comment_chars", "max_comment_body_chars"),
        ("default_note_chars", "max_version_note_chars"),
        ("default_title_chars", "max_title_chars"),
        ("default_summary_chars", "max_summary_chars"),
        ("default_pdf_bytes", "max_pdf_size_bytes"),
        ("default_page_size", "search_page_size"),
        ("default_max_pages", "max_search_pages"),
    ];

    let mut failures = 0;
    for (fn_name, conf_key) in checks {
        let body = fn_body(&limits_src, fn_name)
            .unwrap_or_else(|| panic!("limits.rs fn {fn_name} not found"));
        let front = extract_int_literal(&body)
            .unwrap_or_else(|| panic!("limits.rs {fn_name} body has no int literal: {body}"));
        let conf = conf_u64(&conf, conf_key);
        assert_eq!(
            front, conf,
            "limits.rs {fn_name} = {front} drifted from conf/back/server.toml {conf_key} = {conf}"
        );
        failures += usize::from(front != conf);
    }
    assert_eq!(
        failures, 0,
        "front fallback default(s) drifted from conf/back/server.toml; update limits.rs or conf"
    );
}
