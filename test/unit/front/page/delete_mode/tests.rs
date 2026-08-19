use crate::page::delete_mode::{mode_from_str, mode_to_str};
use nail_common::request::DeleteMode;

const ALL: [DeleteMode; 3] = [DeleteMode::Transfer, DeleteMode::Soft, DeleteMode::Hard];
const SOFT_AND_HARD: [DeleteMode; 2] = [DeleteMode::Soft, DeleteMode::Hard];

#[test]
fn mode_to_str_round_trips_through_mode_from_str() {
    for mode in ALL {
        assert_eq!(mode_from_str(mode_to_str(mode), &ALL), Some(mode));
    }
}

#[test]
fn mode_from_str_rejects_values_outside_the_allowed_set() {
    assert_eq!(mode_from_str("transfer", &SOFT_AND_HARD), None);
    assert_eq!(
        mode_from_str("soft", &SOFT_AND_HARD),
        Some(DeleteMode::Soft)
    );
    assert_eq!(
        mode_from_str("hard", &SOFT_AND_HARD),
        Some(DeleteMode::Hard)
    );
    assert_eq!(mode_from_str("permanent", &ALL), None);
    assert_eq!(mode_from_str("", &ALL), None);
}
