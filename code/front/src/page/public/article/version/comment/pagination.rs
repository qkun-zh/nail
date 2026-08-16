use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::pagination::PrevNext;

pub const COMMENTS_PER_PAGE: u64 = 8;

#[component]
pub fn LevelPagination(
    #[prop(into)] current: Signal<u64>,
    #[prop(into)] has_next: Signal<bool>,
    base_href: String,
) -> impl IntoView {
    let navigate = use_navigate();
    let has_prev = move || current.get() > 1;
    move || {
        let on_go = Callback::new({
            let navigate = navigate.clone();
            let base_href = base_href.clone();
            move |page: u64| {
                navigate(
                    &format!("{base_href}?page={page}"),
                    NavigateOptions {
                        resolve: false,
                        replace: true,
                        ..Default::default()
                    },
                );
            }
        });
        view! {
            <PrevNext current=current has_prev=has_prev has_next=has_next on_go=on_go/>
        }
        .into_any()
    }
}
