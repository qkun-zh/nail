use leptos::prelude::*;
use leptos_router::hooks::query_signal;

/// Keep a router-owned `?key=` param in step with a text field; an empty value
/// drops the param.
pub fn mirror_text_param(key: &'static str, source: impl Fn() -> String + Copy + 'static) {
    let (_, set_param) = query_signal::<String>(key);
    Effect::new(move |_| {
        let value = source();
        set_param.set((!value.is_empty()).then_some(value));
    });
}

/// Keep a router-owned `?key=` param in step with any computed option.
pub fn mirror_param<T>(key: &'static str, source: impl Fn() -> Option<T> + Copy + 'static)
where
    T: Clone + PartialEq + Send + Sync + std::str::FromStr + std::fmt::Display + 'static,
{
    let (_, set_param) = query_signal::<T>(key);
    Effect::new(move |_| {
        set_param.set(source());
    });
}
