
pub const STYLE: &str = r#"
.cmt-section {
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
    max-width: 640px;
}
.cmt-form {
    display: flex;
    flex-direction: column;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    overflow: hidden;
    background: #ffffff;
    margin-bottom: 20px;
    transition: border-color .15s ease, box-shadow .15s ease;
}
.cmt-form:focus-within {
    border-color: #0969da;
    box-shadow: 0 0 0 3px rgba(9, 105, 218, .15);
}
.cmt-input {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border: none;
    font: inherit;
    line-height: 1.5;
    resize: vertical;
    background: transparent;
    color: #1f2328;
}
.cmt-input:focus {
    outline: none;
}
.cmt-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid #eaeef2;
    background: #f6f8fa;
}
.cmt-counter {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-btn {
    padding: 6px 16px;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    background: #ffffff;
    color: #1f2328;
    cursor: pointer;
    font-size: 14px;
    line-height: 1.4;
    transition: background .15s ease, border-color .15s ease;
}
.cmt-btn:hover {
    background: #f6f8fa;
    border-color: #b6bfc9;
}
.cmt-btn:disabled {
    opacity: .5;
    cursor: not-allowed;
}
.cmt-btn-primary {
    background: #1f2328;
    color: #ffffff;
    border-color: #1f2328;
}
.cmt-btn-primary:hover {
    background: #32383f;
    border-color: #32383f;
}
.cmt-btn-danger {
    background: #d1242f;
    color: #ffffff;
    border-color: #d1242f;
}
.cmt-btn-danger:hover {
    background: #b6231c;
    border-color: #b6231c;
}
/* 每条评论上下都有分隔线：列表底边线 + 每项上边线（相邻项只显一条） */
.cmt-list {
    list-style: none;
    margin: 0;
    padding: 0;
    border-bottom: 1px solid #eaeef2;
}
.cmt-item {
    padding: 14px 0;
    border-top: 1px solid #eaeef2;
}
.cmt-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
}
/* name + 时间 + (N) = 一个整体链接，下钻到该评论自己的页面 */
.cmt-meta-link {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    text-decoration: none;
    color: #1f2328;
}
.cmt-meta-link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
    color: #0969da;
}
.cmt-meta-link:hover .cmt-time,
.cmt-meta-link:hover .cmt-count,
.cmt-meta-link:hover .cmt-seq,
.cmt-meta-link:hover .cmt-name {
    color: #0969da;
}
/* 名字与序号同款：小号灰字，不粗不放大 */
.cmt-name {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-time {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-count {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
/* 先来后到的序号，放在 name 前面 */
.cmt-seq {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-body {
    margin: 6px 0 0;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.6;
    color: #1f2328;
}
/* 评论对象卡（评论作为目标时）：与列表有明显区块分界 */
.cmt-context {
    background: #f6f8fa;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    padding: 12px 14px;
    margin-bottom: 16px;
}
.cmt-context .cmt-body {
    margin-top: 8px;
}
.cmt-empty,
.cmt-loading {
    color: #656d76;
    font-size: 14px;
    margin: 8px 0;
}
"#;
