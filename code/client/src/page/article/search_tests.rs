use std::str::FromStr;

use crate::page::article::search::checked_range_subset;

#[test]
fn range_specs_round_trip_through_wire_keys() {
    let mut wire_keys = Vec::new();
    for range in SearchRange::ALL {
        assert_eq!(
            SearchRange::from_str(range.as_str()),
            Ok(range),
            "wire key for {range:?} must parse back to itself",
        );
        wire_keys.push(range.as_str());
    }
    let all_checked = vec![true; SearchRange::ALL.len()];
    assert_eq!(checked_range_subset(&all_checked), wire_keys.join(","));
}

use common::search::SearchRange;
