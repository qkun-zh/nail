use std::str::FromStr;

use crate::page::article::search::{RANGE_SPECS, checked_range_subset};

#[test]
fn range_specs_round_trip_through_wire_keys() {
    let mut wire_keys = Vec::new();
    for spec in RANGE_SPECS {
        assert_eq!(
            SearchRange::from_str(spec.range.as_str()),
            Ok(spec.range),
            "wire key for {:?} must parse back to itself",
            spec.range
        );
        wire_keys.push(spec.range.as_str());
    }
    let all_checked = vec![true; RANGE_SPECS.len()];
    assert_eq!(checked_range_subset(&all_checked), wire_keys.join(","));
}

use common::search::SearchRange;
