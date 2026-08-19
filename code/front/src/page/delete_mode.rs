use leptos::prelude::*;
use nail_common::request::DeleteMode;

pub const ALL_MODES: [DeleteMode; 3] = [DeleteMode::Transfer, DeleteMode::Soft, DeleteMode::Hard];
pub const SOFT_AND_HARD: [DeleteMode; 2] = [DeleteMode::Soft, DeleteMode::Hard];

pub fn mode_to_str(mode: DeleteMode) -> &'static str {
    match mode {
        DeleteMode::Transfer => "transfer",
        DeleteMode::Hard => "hard",
        DeleteMode::Soft => "soft",
    }
}

pub fn mode_from_str(value: &str, allowed: &[DeleteMode]) -> Option<DeleteMode> {
    let mode = match value {
        "transfer" => DeleteMode::Transfer,
        "hard" => DeleteMode::Hard,
        "soft" => DeleteMode::Soft,
        _ => return None,
    };
    allowed.contains(&mode).then_some(mode)
}

#[component]
pub fn DeleteModePicker(
    mode: RwSignal<DeleteMode>,
    name: &'static str,
    allowed: &'static [DeleteMode],
) -> impl IntoView {
    allowed
        .iter()
        .map(|&allowed_mode| {
            let is_selected = move || mode.get() == allowed_mode;
            view! {
                <div>
                    <label>
                        <input
                            type="radio"
                            name=name
                            prop:checked=is_selected
                            on:change=move |_| mode.set(allowed_mode)
                        />
                        {mode_to_str(allowed_mode)}
                    </label>
                </div>
            }
        })
        .collect_view()
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/delete_mode/tests.rs"]
mod tests;
