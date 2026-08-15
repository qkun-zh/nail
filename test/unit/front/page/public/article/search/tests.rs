use crate::page::public::article::search::{default_sort_dir, dir_arrow, sort_label};

#[test]
fn sort_label_passes_known_keys_through() {
    assert_eq!(sort_label("time"), "time");
    assert_eq!(sort_label("title"), "title");
    assert_eq!(sort_label("author"), "author");
}

#[test]
fn sort_label_falls_back_to_the_key() {
    assert_eq!(sort_label("tag"), "tag");
}

#[test]
fn default_sort_dir_is_descending_for_time() {
    assert_eq!(default_sort_dir("time"), "desc");
}

#[test]
fn default_sort_dir_is_ascending_for_other_keys() {
    assert_eq!(default_sort_dir("title"), "asc");
    assert_eq!(default_sort_dir("author"), "asc");
    assert_eq!(default_sort_dir("unknown"), "asc");
}

#[test]
fn dir_arrow_maps_direction_to_an_arrow() {
    assert_eq!(dir_arrow("desc"), "↓");
    assert_eq!(dir_arrow("asc"), "↑");
    assert_eq!(dir_arrow("anything_else"), "↑");
}
