use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::request::tag;

#[component]
pub fn CreateTag() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);
        error.set(None);

        let name = name.get();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match tag::create_tag(&name).await {
                Ok(tag) => {
                    navigate(&format!("/tag/{}", tag.id), Default::default());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Create Tag"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <form on:submit=on_submit>
            <label>
                "Name"
                <input
                    type="text"
                    prop:value=name
                    on:input=move |ev| name.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <button type="submit" disabled=submitting>
                {move || if submitting.get() { "Creating..." } else { "Create" }}
            </button>
        </form>
    }
}
