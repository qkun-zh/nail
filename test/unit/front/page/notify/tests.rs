use crate::page::notify::{
    NotificationType, Toast, capped_insert, remaining_seconds, toast_duration_ms,
};

#[test]
fn error_toasts_last_five_seconds() {
    assert_eq!(toast_duration_ms(NotificationType::Error), 5_000);
    assert_eq!(toast_duration_ms(NotificationType::Success), 3_000);
    assert_eq!(toast_duration_ms(NotificationType::Info), 3_000);
}

#[test]
fn counts_down_remaining_whole_seconds() {
    assert_eq!(remaining_seconds(10_000, 0), 10);
    assert_eq!(remaining_seconds(10_000, 9_500), 1);
    assert_eq!(remaining_seconds(10_000, 9_000), 1);
    assert_eq!(remaining_seconds(10_000, 10_000), 0);
    assert_eq!(remaining_seconds(10_000, 10_500), 0);
}

#[test]
fn history_is_capped_at_one_hundred_newest_entries() {
    let mut history = Vec::new();
    for id in 0..150 {
        capped_insert(
            &mut history,
            Toast {
                id,
                kind: NotificationType::Info,
                message: id.to_string(),
                expires_at_ms: 0,
            },
            100,
        );
    }
    assert_eq!(history.len(), 100);
    assert_eq!(history.first().expect("first").id, 50);
    assert_eq!(history.last().expect("last").id, 149);
}

#[test]
fn history_under_cap_is_untouched() {
    let mut history = Vec::new();
    capped_insert(
        &mut history,
        Toast {
            id: 1,
            kind: NotificationType::Success,
            message: "ok".to_string(),
            expires_at_ms: 0,
        },
        100,
    );
    assert_eq!(history.len(), 1);
}
