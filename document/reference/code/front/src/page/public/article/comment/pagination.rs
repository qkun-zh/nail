
use crate::page::Pagination;
use leptos::prelude::*;

const COMMENTS_PER_PAGE: usize = 8;

pub fn paginate_level<'a>(
    items: &'a [&'a serde_json::Value],
    page: u64,
) -> (Vec<&'a serde_json::Value>, u64) {
    let total = items.len() as u64;
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(COMMENTS_PER_PAGE as u64)
    };
    let start = (page
        .saturating_sub(1)
        .saturating_mul(COMMENTS_PER_PAGE as u64))
    .min(total) as usize;
    let end = (start + COMMENTS_PER_PAGE).min(items.len());
    (items[start..end].to_vec(), total_pages)
}

pub fn level_paginator(page: RwSignal<u64>, total_pages: u64) -> impl IntoView {
    let on_go = Callback::new(move |p: u64| page.set(p));
    view! {
        <Pagination
            current=move || page.get()
            total_pages=move || total_pages
            on_go=on_go
        />
    }
}
