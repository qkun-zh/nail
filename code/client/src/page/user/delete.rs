use common::request::DeleteMode;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::use_confirm_action;
use crate::page::delete_mode::DeleteModePicker;
use crate::page::delete_mode::SOFT_TRANSFER_HARD;
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::panel::{PanelForm, PanelFrame, PanelInner, PanelPage, PanelSubmit, PanelTitle};

#[component]
pub fn DeleteUser() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let mode = RwSignal::new(DeleteMode::Transfer);

    let confirm = use_confirm_action(move || {
        let uid = params.get().get("uid").unwrap_or_default();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        async move {
            require_id(&uid)?;
            crate::request::user::delete_user(&uid, mode.get()).await?;
            notify_success(&notifications, "user deleted");
            navigate(&format!("/user/{uid}"), NavigateOptions::default());
            Ok(())
        }
    });

    view! {
        <PanelPage>
            <PanelFrame>
                <PanelInner>
                    <PanelTitle>"USER DELETE"</PanelTitle>
                    <div class="flex w-full justify-center">
                        <DeleteModePicker mode=mode name="mode" allowed=&SOFT_TRANSFER_HARD/>
                    </div>
                    <PanelForm center=true>
                        <PanelSubmit
                            disabled=confirm.working
                            on_click=Callback::new(move |()| confirm.submit.run(()))
                        >
                            {move || if confirm.working.get() { "deleting..." } else { "delete" }}
                        </PanelSubmit>
                    </PanelForm>
                </PanelInner>
            </PanelFrame>
        </PanelPage>
    }
}
