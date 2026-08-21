use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::request::role;

#[component]
pub fn CreateRole() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let navigate = use_navigate();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if submitting.get() {
            return;
        }
        submitting.set(true);
        error.set(None);

        let name = name.get();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match role::create_role(&name).await {
                Ok(view) => {
                    navigate(&format!("/role/{}", view.id), NavigateOptions::default());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Create Role"</h1>
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
