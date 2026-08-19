use leptos::prelude::*;
use leptos_router::NavigateOptions;

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

pub fn sync_url_on_change<Navigate, Build>(navigate: Navigate, build: Build)
where
    Navigate: Fn(&str, NavigateOptions) + Clone + 'static,
    Build: Fn() -> Option<String> + 'static,
{
    Effect::new(move |previous: Option<()>| {
        let url = build();
        if previous.is_none() {
            return;
        }
        let Some(url) = url else {
            return;
        };
        navigate(
            &url,
            NavigateOptions {
                replace: true,
                resolve: false,
                ..Default::default()
            },
        );
    });
}

pub fn persist_draft<Navigate>(
    navigate: Navigate,
    pathname: String,
    fields: impl Fn() -> Vec<(&'static str, String)> + 'static,
) where
    Navigate: Fn(&str, NavigateOptions) + Clone + 'static,
{
    sync_url_on_change(navigate, move || {
        let captured = fields();
        let pairs: Vec<(&str, &str)> = captured
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        Some(draft_url(&pathname, &pairs))
    });
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/draft/tests.rs"]
mod tests;
