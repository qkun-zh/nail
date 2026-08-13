
use std::time::Duration;

use crate::other::conf::SmtpConfig;
use crate::other::email::{EmailService, SendEmailError};
use crate::unit_tests::context::SMTP_REFUSED_PORT;

fn smtp_on_refused_port() -> SmtpConfig {
    SmtpConfig {
        host: "127.0.0.1".to_string(),
        port: SMTP_REFUSED_PORT,
        username: String::new(),
        password: String::new(),
        from_email: "sender@qq.com".to_string(),
        from_name: String::new(),
        timeout_secs: 1,
        wall_clock_timeout_secs: 2,
        starttls: false,
    }
}

fn is_rate_limited(err: &SendEmailError) -> bool {
    matches!(err, SendEmailError::RateLimited)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rate_window_rejects_second_send_to_same_mailbox() {
    let svc = EmailService::new(smtp_on_refused_port(), 3600);
    let to = "alice@qq.com";

    let first = svc.send_email(to, "s1", "b").await;
    assert!(
        first.is_err() && !is_rate_limited(first.as_ref().err().unwrap()),
        "首次发送不应命中速率（真实投递失败而非限速）"
    );

    let second = svc.send_email(to, "s2", "b").await;
    let err = second.err().expect("窗口内重复发送必须被拒");
    assert!(
        is_rate_limited(&err),
        "窗口内重复发送必须返回 RateLimited，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rate_window_passes_after_cooldown_elapses() {
    let svc = EmailService::new(smtp_on_refused_port(), 1);
    let to = "bob@qq.com";

    svc.send_email(to, "s1", "b")
        .await
        .expect_err("SMTP 必失败");

    tokio::time::sleep(Duration::from_millis(1200)).await;

    let after = svc.send_email(to, "s2", "b").await;
    let err = after.err().expect("SMTP 必失败");
    assert!(
        !is_rate_limited(&err),
        "窗口外放行：应走到真实投递（SMTP 失败）而非速率拒绝，实际: {err:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rate_is_per_mailbox_not_global() {
    let svc = EmailService::new(smtp_on_refused_port(), 3600);
    svc.send_email("carol@qq.com", "s1", "b")
        .await
        .expect_err("SMTP 必失败");
    let other = svc.send_email("dave@qq.com", "s2", "b").await;
    let err = other.err().expect("SMTP 必失败");
    assert!(
        !is_rate_limited(&err),
        "不同收件人不应共享速率窗口: {err:?}"
    );
}
