use crate::page::notify::{NotificationType, TOAST_DURATION_MS, kind_class};

#[test]
fn toasts_dismiss_after_four_seconds() {
    assert_eq!(TOAST_DURATION_MS, 4_000);
}

#[test]
fn every_kind_maps_to_its_own_css_class() {
    assert_eq!(kind_class(NotificationType::Success), "success");
    assert_eq!(kind_class(NotificationType::Error), "error");
}
