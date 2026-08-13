
use std::path::PathBuf;

use uuid::Uuid;

use crate::other::pdf::{GuardError, PdfStreamGuard, PdfUpload, TempPdf};

fn guard_check(data: &[u8], max: u64) -> Result<(), GuardError> {
    let mut guard = PdfStreamGuard::new(max);
    for chunk in data.chunks(7) {
        guard.update(chunk)?;
    }
    guard.finish()
}

fn temp_file(tag: &str, content: &[u8]) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("nail_pdf_upload_{tag}_{}", Uuid::now_v7()));
    std::fs::create_dir_all(&dir).expect("create upload tmp dir");
    let path = dir.join("stage.pdf");
    std::fs::write(&path, content).expect("write temp file");
    (dir, path)
}

#[test]
fn valid_pdf_passes() {
    let pdf = crate::unit_tests::context::test_pdf();
    assert!(
        guard_check(&pdf, 1024 * 1024).is_ok(),
        "最小合法 PDF 必须通过"
    );
}

#[test]
fn too_large_rejected() {
    let pdf = crate::unit_tests::context::test_pdf();
    let err = guard_check(&pdf, 10).expect_err("超限必须拒绝");
    assert!(err.to_string().contains("PDF too large"), "实际: {err}");
}

#[test]
fn too_small_rejected() {
    let err = guard_check(b"%PDF-123", 1024).expect_err("过小必须拒绝");
    assert!(err.to_string().contains("PDF too small"), "实际: {err}");
    let err = guard_check(b"", 1024).expect_err("空文件必须拒绝");
    assert!(err.to_string().contains("PDF too small"));
}

#[test]
fn wrong_header_rejected() {
    let mut bad = crate::unit_tests::context::test_pdf();
    bad[0] = b'X';
    let err = guard_check(&bad, 1024 * 1024).expect_err("坏头部必须拒绝");
    assert!(
        err.to_string().contains("Invalid PDF header"),
        "实际: {err}"
    );
}

#[test]
fn bad_version_rejected() {
    let err = guard_check(b"%PDF-3.0\n1234567\n%%EOF\n", 1024).expect_err("非法版本必须拒绝");
    assert!(
        err.to_string().contains("Invalid PDF version"),
        "实际: {err}"
    );
}

#[test]
fn missing_eof_rejected() {
    let mut bad = crate::unit_tests::context::test_pdf();
    bad.truncate(bad.len() - 5);
    let err = guard_check(&bad, 1024 * 1024).expect_err("缺 EOF 必须拒绝");
    assert!(
        err.to_string().contains("Invalid PDF footer"),
        "实际: {err}"
    );
}

#[test]
fn trailing_garbage_after_eof_rejected() {
    let mut bad = crate::unit_tests::context::test_pdf();
    bad.extend_from_slice(b"<script>alert(1)</script>");
    let err = guard_check(&bad, 1024 * 1024).expect_err("EOF 后尾随垃圾必须拒绝");
    assert!(
        err.to_string().contains("Invalid PDF footer"),
        "实际: {err}"
    );
}

#[test]
fn whitespace_after_eof_allowed() {
    let mut ok = crate::unit_tests::context::test_pdf();
    ok.extend_from_slice(b"  \t\r\n\n");
    assert!(guard_check(&ok, 1024 * 1024).is_ok(), "EOF 后允许空白");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn received_drop_removes_tmp() {
    let (dir, path) = temp_file("received", b"%PDF-1.4\n%%EOF\n");
    let upload = PdfUpload::received(common::hash::pdf(b"x"), TempPdf::new(path.clone()));
    assert!(path.is_file(), "drop 前临时文件存在");
    drop(upload);
    assert!(!path.exists(), "Received 态 drop 必须删除临时文件");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn placed_drop_removes_final() {
    let (dir, path) = temp_file("placed", b"%PDF-1.4\n%%EOF\n");
    let final_path = dir.join("final.pdf");
    let upload = PdfUpload::received(common::hash::pdf(b"x"), TempPdf::new(path.clone()));
    let upload = upload
        .place(final_path.clone())
        .await
        .expect("place 必须成功");
    assert!(final_path.is_file(), "place 后最终文件存在");
    drop(upload);
    assert!(!final_path.exists(), "Placed 态 drop 必须删除最终文件");
    assert!(!path.exists(), "临时文件已被 rename 走并由 TempPdf 清理");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn kept_preserves_file() {
    let (dir, path) = temp_file("kept", b"%PDF-1.4\n%%EOF\n");
    let final_path = dir.join("final.pdf");
    let upload = PdfUpload::received(common::hash::pdf(b"x"), TempPdf::new(path.clone()));
    let upload = upload.place(final_path.clone()).await.expect("place");
    let upload = upload.keep_final();
    drop(upload);
    assert!(final_path.is_file(), "Kept 态 drop 必须保留最终文件");
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn place_skips_rename_when_target_exists() {
    let (dir, path) = temp_file("skip", b"%PDF-1.4\n%%EOF\nTMP CONTENT");
    let final_path = dir.join("final.pdf");
    std::fs::write(&final_path, b"CONTENT A").expect("写目标文件");
    let upload = PdfUpload::received(common::hash::pdf(b"x"), TempPdf::new(path.clone()));
    let upload = upload
        .place(final_path.clone())
        .await
        .expect("place 跳过 rename");
    assert_eq!(
        std::fs::read(&final_path).expect("读目标文件"),
        b"CONTENT A",
        "place 不得覆盖已存在目标"
    );
    assert!(!path.exists(), "临时文件必须被清理");
    drop(upload);
    let _ = std::fs::remove_dir_all(&dir);
}
