use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use nail_common::request::DeleteMode;

fn mode_from_str(value: &str) -> Option<DeleteMode> {
    match value {
        "soft" => Some(DeleteMode::Soft),
        "hard" => Some(DeleteMode::Hard),
        _ => None,
    }
}

#[component]
pub fn DeleteVersion() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let navigate = use_navigate();

    let working = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let mode = RwSignal::new(
        query
            .get_untracked()
            .get("mode")
            .and_then(|value| mode_from_str(&value))
            .unwrap_or(DeleteMode::Soft),
    );

    let version_id = move || params.get().get("version_id").unwrap_or_default();
    let article_id = move || params.get().get("article_id").unwrap_or_default();

    let on_delete = Callback::new(move |()| {
        if working.get() {
            return;
        }
        working.set(true);
        error.set(None);
        let version_id = version_id();
        let article_id = article_id();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match crate::request::version::delete_version(&version_id, mode.get()).await {
                Ok(_) => {
                    navigate(
                        &format!("/article/{article_id}/version"),
                        NavigateOptions::default(),
                    );
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    working.set(false);
                }
            }
        });
    });

    view! {
        <h1>"Delete Version"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <div>
            <label>
                <input
                    type="radio"
                    name="version_delete_mode"
                    prop:checked=move || mode.get() == DeleteMode::Soft
                    on:change=move |_| mode.set(DeleteMode::Soft)
                />
                "soft"
            </label>
        </div>
        <div>
            <label>
                <input
                    type="radio"
                    name="version_delete_mode"
                    prop:checked=move || mode.get() == DeleteMode::Hard
                    on:change=move |_| mode.set(DeleteMode::Hard)
                />
                "hard"
            </label>
        </div>
        <button on:click=move |_| on_delete.run(()) disabled=move || working.get()>
            {move || if working.get() { "deleting..." } else { "delete" }}
        </button>
    }
}
