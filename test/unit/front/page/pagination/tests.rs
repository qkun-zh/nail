use crate::page::pagination::{PaginationState, clamp_page_size, pagination_state};

#[test]
fn next_page_is_driven_only_by_the_server_has_next_flag() {
    assert_eq!(
        pagination_state(1, true),
        PaginationState {
            page: 1,
            previous_page: None,
            next_page: Some(2)
        }
    );
    assert_eq!(
        pagination_state(1, false),
        PaginationState {
            page: 1,
            previous_page: None,
            next_page: None
        }
    );
    assert_eq!(
        pagination_state(3, true),
        PaginationState {
            page: 3,
            previous_page: Some(2),
            next_page: Some(4)
        }
    );
    assert_eq!(
        pagination_state(3, false),
        PaginationState {
            page: 3,
            previous_page: Some(2),
            next_page: None
        }
    );
}

#[test]
fn page_is_clamped_to_at_least_one() {
    assert_eq!(
        pagination_state(0, true),
        PaginationState {
            page: 1,
            previous_page: None,
            next_page: Some(2)
        }
    );
}

#[test]
fn clamps_page_size_to_the_backend_range() {
    assert_eq!(clamp_page_size(0, 8), 8);
    assert_eq!(clamp_page_size(5, 8), 5);
    assert_eq!(clamp_page_size(1, 8), 1);
    assert_eq!(clamp_page_size(200, 8), 200);
    assert_eq!(clamp_page_size(201, 8), 200);
    assert_eq!(clamp_page_size(9999, 8), 200);
}
