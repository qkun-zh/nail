use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::query_signal_with_options;

/// Keep a router-owned `?key=` param in step with a text field; an empty value
/// drops the param. Uses replaceState to avoid polluting history stack.
pub fn mirror_text_param(key: &'static str, source: impl Fn() -> String + Copy + 'static) {
    let options = NavigateOptions {
        replace: true,
        ..Default::default()
    };
    let (_, set_param) = query_signal_with_options::<String>(key, options);
    Effect::new(move |_| {
        let value = source();
        set_param.set((!value.is_empty()).then_some(value));
    });
}

/// Keep a router-owned `?key=` param in step with any computed option.
/// Uses replaceState to avoid polluting history stack.
pub fn mirror_param<T>(key: &'static str, source: impl Fn() -> Option<T> + Copy + 'static)
where
    T: Clone + PartialEq + Send + Sync + std::str::FromStr + std::fmt::Display + 'static,
{
    let options = NavigateOptions {
        replace: true,
        ..Default::default()
    };
    let (_, set_param) = query_signal_with_options::<T>(key, options);
    Effect::new(move |_| {
        set_param.set(source());
    });
}
