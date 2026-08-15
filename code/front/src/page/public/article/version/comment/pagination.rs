use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::pagination::Pagination;

pub const COMMENTS_PER_PAGE: u64 = 8;

#[component]
pub fn LevelPagination(
    #[prop(into)] current: Signal<u64>,
    #[prop(into)] total_pages: Signal<u64>,
    base_href: String,
) -> impl IntoView {
    let navigate = use_navigate();
    move || {
        if total_pages.get() <= 1 {
            return ().into_any();
        }
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
            <Pagination current=current total_pages=total_pages on_go=on_go/>
        }
        .into_any()
    }
}
