use crate::page::pagination::{clamp_local_page, clamp_page_size, local_page_count, local_page_of};

#[test]
fn clamps_page_size_to_the_backend_range() {
    assert_eq!(clamp_page_size(0, 8), 8);
    assert_eq!(clamp_page_size(5, 8), 5);
    assert_eq!(clamp_page_size(1, 8), 1);
    assert_eq!(clamp_page_size(200, 8), 200);
    assert_eq!(clamp_page_size(201, 8), 200);
    assert_eq!(clamp_page_size(9999, 8), 200);
}

#[test]
fn local_page_count_uses_the_given_page_size() {
    assert_eq!(local_page_count(0, 8), 1);
    assert_eq!(local_page_count(1, 8), 1);
    assert_eq!(local_page_count(8, 8), 1);
    assert_eq!(local_page_count(9, 8), 2);
    assert_eq!(local_page_count(16, 8), 2);
    assert_eq!(local_page_count(17, 8), 3);
    assert_eq!(local_page_count(10, 4), 3);
}

#[test]
fn clamp_local_page_stays_within_range() {
    assert_eq!(clamp_local_page(0, 1), 1);
    assert_eq!(clamp_local_page(1, 3), 1);
    assert_eq!(clamp_local_page(2, 3), 2);
    assert_eq!(clamp_local_page(3, 3), 3);
    assert_eq!(clamp_local_page(9, 3), 3);
}

#[test]
fn local_page_of_groups_by_the_given_page_size() {
    assert_eq!(local_page_of(0, 8), 0);
    assert_eq!(local_page_of(7, 8), 0);
    assert_eq!(local_page_of(8, 8), 1);
    assert_eq!(local_page_of(15, 8), 1);
    assert_eq!(local_page_of(16, 8), 2);
    assert_eq!(local_page_of(4, 4), 1);
}
