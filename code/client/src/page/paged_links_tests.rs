use crate::page::paged_links::{clamp_page, current_page, total_pages};

#[test]
fn total_pages_ceil_division_with_a_minimum_of_one() {
    assert_eq!(total_pages(0, 8), 1);
    assert_eq!(total_pages(1, 8), 1);
    assert_eq!(total_pages(8, 8), 1);
    assert_eq!(total_pages(9, 8), 2);
    assert_eq!(total_pages(16, 8), 2);
    assert_eq!(total_pages(17, 8), 3);
    assert_eq!(total_pages(10, 4), 3);
}

#[test]
fn current_page_defaults_to_one_and_never_drops_below_one() {
    assert_eq!(current_page(None), 1);
    assert_eq!(current_page(Some(0)), 1);
    assert_eq!(current_page(Some(1)), 1);
    assert_eq!(current_page(Some(3)), 3);
}

#[test]
fn clamp_page_stays_within_the_page_range() {
    assert_eq!(clamp_page(0, 1), 1);
    assert_eq!(clamp_page(1, 5), 1);
    assert_eq!(clamp_page(3, 5), 3);
    assert_eq!(clamp_page(5, 5), 5);
    assert_eq!(clamp_page(9, 5), 5);
}
