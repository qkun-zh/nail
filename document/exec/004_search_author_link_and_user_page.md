# 004 — 搜索作者链接 + 用户公开页

## 1. Requirement

搜索结果中的作者名（文章作者 + 评论作者）可点击跳转 `/user/{uid}`；新增 `/user/{uid}` 公开页面显示用户信息+文章列表；所有操作按钮对所有人可见，后端鉴权。

**验收标准**：
1. 搜索结果中文章作者名是链接 → `/user/{article_author_id}`
2. 搜索结果中评论作者名是链接 → `/user/{comment_author_id}`
3. `/user/{uid}` 页面显示 id、name、email_hash、roles、articles
4. 所有现有操作按钮不因登录状态隐藏
5. 无权限时后端返回403，前端通知"权限不足"

## 2. Scope

**In**: common 类型、back search index、back logic、front 搜索渲染、front 用户页面、front 按钮可见性
**Out**: 不删除 /admin 路由、不改权限模型

## 3. Design Decisions

- author_id 存入搜索索引（零运行时开销，仅一次启动 rebuild）
- /user/{uid} 页面复用 admin 详情页逻辑
- 操作按钮对所有人可见，后端 403 时前端通知

## 4. Slice Breakdown

| Slice | Goal | Files |
|---|---|---|
| S1 | common: SearchArticleItem + SearchCommentItem 加 author_id | `common/src/response/search.rs` |
| S2 | back: search index 加 FIELD_AUTHOR_ID | `repository/search/schema.rs`, `document.rs`, `search.rs` |
| S3 | back: logic/search.rs 传递 author_id | `logic/search.rs` |
| S4 | front: 搜索结果作者名改链接 | `page/public/article/search/results.rs`, `comments.rs` |
| S5 | front: 新增 /user/{uid} 公开页面 + 路由 | `page/user.rs`, `router.rs` |
| S6 | front: 去掉操作按钮的登录隐藏 | `page/public/article/detail.rs` 等 |

## 5. Open Unknowns

- search index schema version 升级后自动 rebuild 行为 — 已有机制（`INDEX_SCHEMA_VERSION`），source 确认

## 6. Verification Plan

| Dimension | Method |
|---|---|
| Correctness | cargo test (513 back + 69 front) |
| Behavior change | trunk build + 手动验证搜索结果链接 |
| Time complexity | 无新增运行时开销（索引存储） |
| Space complexity | ~36 bytes/document 额外存储 |
| Performance | 无回退（索引读取 vs 原有） |

## 7. Risks

- 搜索索引 rebuild 首次启动稍慢 — 可接受
- 旧索引自动删除重建 — 已有机制

## 8. Constraints

- 不改权限模型
- 不删除 /admin 路由

## 9. Questions

无
