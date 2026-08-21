#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn js_number_to_u64(value: f64) -> u64 {
    value as u64
}
