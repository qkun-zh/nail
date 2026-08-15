const JS_DATE_MAX_MS: f64 = 8.64e15;

pub fn format_iso8601(unix_secs: u64) -> String {
    if unix_secs == 0 {
        return String::new();
    }
    let shifted_ms = unix_secs as f64 * 1000.0 + 8.0 * 3600.0 * 1000.0;
    if shifted_ms > JS_DATE_MAX_MS {
        return String::new();
    }
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(shifted_ms));
    let iso = date.to_iso_string().as_string().unwrap_or_default();
    iso.split_once('.')
        .map(|(head, _)| format!("{}Z", head))
        .unwrap_or(iso)
        .replace('Z', "+08:00")
}
