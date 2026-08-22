use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::page::validation::validate_uuid;
use crate::request::tag::{self, NamedRef};

#[component]
pub fn TagDetail() -> impl IntoView {
    let params = use_params_map();
    let tag = RwSignal::new(None::<NamedRef>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        if let Err(error_message) = validate_uuid(&tag_id) {
            error.set(Some(error_message));
            return;
        }
        leptos::task::spawn_local(async move {
            match tag::read_tag(&tag_id).await {
                Ok(tag_view) => tag.set(Some(tag_view)),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(tag_view) = tag.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };
        let tag_id = tag_view.id.clone();
        let update_href = format!("/tag/{tag_id}/update");
        let delete_href = format!("/tag/{tag_id}/delete");
        view! {
            <div>
                <h1>"Tag: " {tag_view.name}</h1>
                <hr/>
                <div><A href=update_href>"update"</A></div>
                <hr/>
                <div><A href=delete_href>"delete"</A></div>
                <hr/>
            </div>
        }
        .into_any()
    };

    view! { {render} }
}
