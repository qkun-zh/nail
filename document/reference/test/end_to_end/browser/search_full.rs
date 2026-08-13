
use crate::end_to_end_tests::browser::context::{BROWSER_SERIAL, EndToEndBrowserContext};

fn unique_email(prefix: &str) -> String {
    format!("{prefix}_{}@qq.com", uuid::Uuid::now_v7())
}

async fn seed(
    ctx: &EndToEndBrowserContext,
    session: &str,
) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    let marker = format!("Search{}", uuid::Uuid::now_v7());
    let alpha_title = format!("{marker} memory safety");
    let beta_title = format!("{marker} unrelated");
    let gamma_title = format!("{marker} memory alone");
    let (alpha, _) = ctx
        .create_article(
            session,
            &alpha_title,
            "rust internals here",
            "#alpha",
            "1.0.0",
            "note alpha",
        )
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (beta, _) = ctx
        .create_article(
            session,
            &beta_title,
            "memory in summary",
            "#beta",
            "1.0.0",
            "note beta",
        )
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (gamma, gamma_v) = ctx
        .create_article(
            session,
            &gamma_title,
            "plain summary",
            "#gamma",
            "1.0.0",
            "note gamma",
        )
        .await;
    (
        alpha,
        beta,
        gamma,
        marker,
        alpha_title,
        beta_title,
        gamma_title,
        gamma_v,
    )
}

#[tokio::test]
async fn url_multiword_and_and_cross_range_or() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("surl")).await;
    ctx.set_session_token(&session).await;
    let (_a, _b, _g, _marker, alpha_title, beta_title, gamma_title, _v) =
        seed(&ctx, &session).await;
    let u = |query: &str| format!("{}/public/article/search?{}", ctx.base_url, query);

    ctx.page
        .goto(&u(&format!("q=memory+safety&ranges=title")))
        .await
        .expect("goto AND");
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    ctx.wait_for_text(&alpha_title, 10).await;
    let body = ctx.body_text().await;
    assert!(
        !body.contains(&beta_title),
        "AND leaked beta (summary-only)"
    );
    assert!(
        !body.contains(&gamma_title),
        "AND leaked gamma (single term)"
    );

    ctx.page
        .goto(&u("q=memory&ranges=title,summary"))
        .await
        .expect("goto OR");
    ctx.wait_for_text(&alpha_title, 10).await;
    let body = ctx.body_text().await;
    assert!(body.contains(&beta_title), "OR missing beta (summary hit)");
    assert!(body.contains("标题"), "title hit should carry 标题 label");
    assert!(body.contains("摘要"), "summary hit should carry 摘要 label");
}

#[tokio::test]
async fn url_tag_scope_and_empty_result() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("stag")).await;
    ctx.set_session_token(&session).await;
    let (_a, _b, _g, _marker, alpha_title, beta_title, _gamma_title, _v) =
        seed(&ctx, &session).await;
    let u = |query: &str| format!("{}/public/article/search?{}", ctx.base_url, query);

    ctx.page
        .goto(&u(&format!("q=alpha&ranges=tag")))
        .await
        .expect("goto tag search");
    ctx.wait_for_text(&alpha_title, 10).await;
    let body = ctx.body_text().await;
    assert!(!body.contains(&beta_title), "tag scope leaked beta");
    assert!(body.contains("标签"), "tag hit should carry 标签 label");

    ctx.page
        .goto(&u(&format!("q=NoSuchTermXYZ&ranges=title")))
        .await
        .expect("goto empty");
    ctx.wait_for_text("none", 10).await;
}

#[tokio::test]
async fn url_sort_and_pagination_and_time_window() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("ssort")).await;
    ctx.set_session_token(&session).await;
    let (_a, _b, _g, _marker, alpha_title, beta_title, gamma_title, gamma_v) =
        seed(&ctx, &session).await;
    let u = |query: &str| format!("{}/public/article/search?{}", ctx.base_url, query);

    ctx.page
        .goto(&u(&format!(
            "q=memory&ranges=title,summary&sort=title:asc&limit=10&page=1"
        )))
        .await
        .expect("goto title sort");
    ctx.wait_for_text(&alpha_title, 10).await;
    let body = ctx.body_text().await;
    assert!(body.contains(&beta_title), "beta in sorted list");
    assert!(body.contains(&gamma_title), "gamma in sorted list");

    let t_gamma = common::time::uuidv7_timestamp_secs(&gamma_v).expect("uuidv7");
    ctx.page
        .goto(&u(&format!(
            "q=memory&ranges=title,summary&from={t_gamma}&to={t_gamma}"
        )))
        .await
        .expect("goto time window");
    let body = {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let body = ctx.body_text().await;
            if body.contains(&gamma_title) && !body.contains(&alpha_title) {
                break body;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "time window never converged; body: {:?}",
                body.chars().take(600).collect::<String>()
            );
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    };
    assert!(!body.contains(&alpha_title), "time window leaked alpha");
    assert!(!body.contains(&beta_title), "time window leaked beta");

    ctx.page
        .goto(&u(&format!(
            "q=memory&ranges=title,summary&sort=title:asc&limit=1&page=1"
        )))
        .await
        .expect("goto page1");
    {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let body = ctx.body_text().await;
            if body.contains(&gamma_title) && !body.contains(&alpha_title) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "page1 never converged; body: {:?}",
                body.chars().take(600).collect::<String>()
            );
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    ctx.page
        .goto(&u(&format!(
            "q=memory&ranges=title,summary&sort=title:asc&limit=1&page=2"
        )))
        .await
        .expect("goto page2");
    {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let body = ctx.body_text().await;
            if body.contains(&alpha_title) && !body.contains(&gamma_title) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "page2 never converged; body: {:?}",
                body.chars().take(600).collect::<String>()
            );
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

#[tokio::test]
async fn ui_search_input_checkbox_and_sort() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("sui")).await;
    ctx.set_session_token(&session).await;
    let (_a, _b, _g, _marker, alpha_title, gamma_title, _beta_title, _v) =
        seed(&ctx, &session).await;
    let base = format!("{}/public/article/search", ctx.base_url);

    ctx.page.goto(&base).await.expect("goto search page");
    ctx.type_retry("input[type=text]", "memory safety", "search box")
        .await;
    ctx.press_enter_retry("input[type=text]", "search box")
        .await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let body = ctx.body_text().await;
        if body.contains(&alpha_title) && !body.contains(&gamma_title) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "multiword AND result never converged; body: {:?}",
            body.chars().take(600).collect::<String>()
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let body = ctx.body_text().await;
    assert!(!body.contains(&gamma_title), "multiword AND excluded gamma");

    let js = r#"
        (() => {
            const btns = Array.from(document.querySelectorAll('button[type="button"]'));
            const b = btns.find(el => el.textContent.trim() === '标题字母序');
            if (b) { b.click(); return true; }
            return false;
        })()
    "#;
    let clicked: bool = ctx
        .page
        .evaluate(js)
        .await
        .expect("evaluate sort click")
        .into_value()
        .expect("sort click value");
    assert!(clicked, "title sort button not found");
    ctx.wait_for_text(&alpha_title, 10).await;
}

async fn wait_for_url_fragment(ctx: &EndToEndBrowserContext, needle: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let url = ctx
            .page
            .url()
            .await
            .expect("read page url")
            .unwrap_or_default();
        if url.contains(needle) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "URL never contained {needle:?}; url so far: {url}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn wait_for_gone(ctx: &EndToEndBrowserContext, gone: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let text = ctx.body_text().await;
        if !text.contains(gone) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "body never dropped {gone:?}; body so far: {:?}",
            text.chars().take(400).collect::<String>()
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn url_time_window_from_to() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("swin")).await;
    ctx.set_session_token(&session).await;
    let marker = format!("Win{}", uuid::Uuid::now_v7());
    let first_title = format!("{marker} first");
    let second_title = format!("{marker} second");
    let (_first, first_v) = ctx
        .create_article(&session, &first_title, "s", "#w", "1.0.0", "n")
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (_second, second_v) = ctx
        .create_article(&session, &second_title, "s", "#w", "1.0.0", "n")
        .await;
    let t1 = common::time::uuidv7_timestamp_secs(&first_v).expect("first_v uuidv7");
    let t2 = common::time::uuidv7_timestamp_secs(&second_v).expect("second_v uuidv7");
    assert!(t1 < t2, "seed 间隔必须让两篇落在不同秒: {t1} vs {t2}");
    let u = |query: &str| format!("{}/public/article/search?{}", ctx.base_url, query);

    ctx.page
        .goto(&u(&format!("from={t1}&to={t1}")))
        .await
        .expect("goto window");
    ctx.wait_for_text(&first_title, 10).await;
    let body = ctx.body_text().await;
    assert!(!body.contains(&second_title), "to=t1 窗口泄漏了第二篇");

    ctx.page
        .goto(&u(&format!("from={t2}")))
        .await
        .expect("goto from t2");
    ctx.wait_for_text(&second_title, 10).await;
    let body = ctx.body_text().await;
    assert!(!body.contains(&first_title), "from=t2 窗口泄漏了第一篇");

    ctx.page.reload().await.expect("reload window");
    ctx.wait_for_text(&second_title, 10).await;
    let body = ctx.body_text().await;
    assert!(
        !body.contains(&first_title),
        "reload 后 from=t2 窗口泄漏了第一篇"
    );
}

#[tokio::test]
async fn url_relevance_order_and_mark_highlight() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("srel")).await;
    ctx.set_session_token(&session).await;
    let marker = format!("Rel{}", uuid::Uuid::now_v7());
    let alpha_title = format!("{marker} needle needle alpha");
    let gamma_title = format!("{marker} needle alone gamma");
    let (_alpha, _) = ctx
        .create_article(
            &session,
            &alpha_title,
            "needle in summary",
            "#r",
            "1.0.0",
            "n",
        )
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (_gamma, _) = ctx
        .create_article(&session, &gamma_title, "plain summary", "#r", "1.0.0", "n")
        .await;
    let u = format!(
        "{}/public/article/search?q=needle&ranges=title,summary",
        ctx.base_url
    );

    ctx.page.goto(&u).await.expect("goto relevance");
    ctx.wait_for_text(&alpha_title, 10).await;
    ctx.wait_for_text(&gamma_title, 10).await;
    let body = ctx.body_text().await;
    assert!(
        body.find(&alpha_title).expect("alpha 在列") < body.find(&gamma_title).expect("gamma 在列"),
        "双来源命中的 alpha 必须排在单来源命中的 gamma 前（相关度优先于时间）"
    );
    assert!(body.contains("[标题]"), "title hit 应带 [标题] 标签");
    assert!(body.contains("[摘要]"), "summary hit 应带 [摘要] 标签");
    let marked: serde_json::Value = ctx
        .page
        .evaluate(
            "(() => { const m = document.querySelector('mark'); return m ? m.textContent : null; })()",
        )
        .await
        .expect("evaluate mark")
        .into_value()
        .expect("mark value");
    assert_eq!(
        marked.as_str(),
        Some("needle"),
        "命中词必须渲染成 <mark>needle</mark>，实际 {marked:?}"
    );
}

#[tokio::test]
async fn url_author_sort_from_url() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session_a = ctx.login_via_api(&unique_email("saut")).await;
    let session_b = ctx.login_via_api(&unique_email("saut")).await;
    ctx.set_session_token(&session_a).await;
    let name_a = read_author_name(&ctx, &session_a).await;
    let name_b = read_author_name(&ctx, &session_b).await;
    assert_ne!(name_a, name_b, "两个新用户的名字必须不同");
    let (article_a, _) = ctx
        .create_article(&session_a, "Author Alpha", "s", "#a", "1.0.0", "n")
        .await;
    let (article_b, _) = ctx
        .create_article(&session_b, "Author Beta", "s", "#b", "1.0.0", "n")
        .await;
    let asc_first = if name_a < name_b {
        (article_a.clone(), name_a.clone())
    } else {
        (article_b.clone(), name_b.clone())
    };
    let asc_second = if name_a < name_b {
        (article_b, name_b)
    } else {
        (article_a, name_a)
    };
    let u = |query: &str| format!("{}/public/article/search?{}", ctx.base_url, query);

    ctx.page
        .goto(&u("sort=author:asc"))
        .await
        .expect("goto author asc");
    ctx.wait_for_text(&asc_first.1, 10).await;
    let body = ctx.body_text().await;
    assert!(
        body.find(&asc_first.1).expect("小名次作者在列")
            < body.find(&asc_second.1).expect("大名次作者在列"),
        "author:asc 必须小字典序作者在前"
    );

    ctx.page
        .goto(&u("sort=author:desc"))
        .await
        .expect("goto author desc");
    ctx.wait_for_text(&asc_second.1, 10).await;
    let body = ctx.body_text().await;
    assert!(
        body.find(&asc_second.1).expect("大名次作者在列")
            < body.find(&asc_first.1).expect("小名次作者在列"),
        "author:desc 必须大字典序作者在前"
    );
}

async fn read_author_name(ctx: &EndToEndBrowserContext, session: &str) -> String {
    let resp = ctx
        .client
        .get(format!("{}/api/user/name", ctx.base_url))
        .header("nail-token", session)
        .send()
        .await
        .expect("GET user/name");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let json: serde_json::Value = resp.json().await.expect("user/name json");
    json["name"].as_str().expect("name present").to_string()
}

#[tokio::test]
async fn ui_range_checkbox_and_sort_pool() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("suirc")).await;
    ctx.set_session_token(&session).await;
    let (_a, _b, _g, _marker, alpha_title, beta_title, gamma_title, _v) =
        seed(&ctx, &session).await;
    let base = format!("{}/public/article/search", ctx.base_url);

    ctx.page.goto(&base).await.expect("goto search");
    ctx.type_retry("input[type=text]", "memory", "search box")
        .await;
    ctx.press_enter_retry("input[type=text]", "search box")
        .await;
    ctx.wait_for_text(&alpha_title, 10).await;
    ctx.wait_for_text(&beta_title, 10).await;
    ctx.wait_for_text(&gamma_title, 10).await;

    let unchecked: bool = ctx
        .page
        .evaluate(
            "(() => { const l = Array.from(document.querySelectorAll('label')).find(el => el.textContent.trim() === '标题'); if (!l) return false; const cb = l.querySelector('input[type=checkbox]'); if (!cb || !cb.checked) return false; cb.click(); return true; })()",
        )
        .await
        .expect("evaluate uncheck title")
        .into_value()
        .expect("uncheck title value");
    assert!(unchecked, "标题 checkbox 未找到或未勾选");
    wait_for_url_fragment(&ctx, "ranges=").await;
    ctx.wait_for_text(&beta_title, 10).await;
    wait_for_gone(&ctx, &alpha_title).await;
    wait_for_gone(&ctx, &gamma_title).await;
    let url = ctx.page.url().await.expect("page url").unwrap_or_default();
    assert!(
        !url.contains("title"),
        "取消标题后 URL 的 ranges 不得再含 title: {url}"
    );

    let rechecked: bool = ctx
        .page
        .evaluate(
            "(() => { const l = Array.from(document.querySelectorAll('label')).find(el => el.textContent.trim() === '标题'); if (!l) return false; const cb = l.querySelector('input[type=checkbox]'); if (!cb || cb.checked) return false; cb.click(); return true; })()",
        )
        .await
        .expect("evaluate recheck title")
        .into_value()
        .expect("recheck title value");
    assert!(rechecked, "标题 checkbox 未找到或未取消");
    ctx.wait_for_text(&alpha_title, 10).await;
    ctx.wait_for_text(&gamma_title, 10).await;

    let js_click_pool = |label: &str| {
        format!(
            "(() => {{ const b = Array.from(document.querySelectorAll('button[type=\"button\"]')).find(el => el.textContent.trim() === {label:?}); if (!b) return false; b.click(); return true; }})()"
        )
    };
    let clicked: bool = ctx
        .page
        .evaluate(js_click_pool("标题字母序"))
        .await
        .expect("evaluate add title sort")
        .into_value()
        .expect("add title sort value");
    assert!(clicked, "标题字母序 池按钮未找到");
    wait_for_url_fragment(&ctx, "sort=title%3Aasc").await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let body = ctx.body_text().await;
        let ok = body.find(&gamma_title).is_some()
            && body.find(&gamma_title).expect("gamma") < body.find(&alpha_title).expect("alpha")
            && body.find(&alpha_title).expect("alpha") < body.find(&beta_title).expect("beta");
        if ok {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "title asc 排序未收敛; body: {:?}",
            body.chars().take(300).collect::<String>()
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let toggled: bool = ctx
        .page
        .evaluate(
            "(() => { const b = Array.from(document.querySelectorAll('button[type=\"button\"]')).find(el => el.textContent.trim() === '↑'); if (!b) return false; b.click(); return true; })()",
        )
        .await
        .expect("evaluate toggle dir")
        .into_value()
        .expect("toggle dir value");
    assert!(toggled, "方向按钮 ↑ 未找到");
    wait_for_url_fragment(&ctx, "sort=title%3Adesc").await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let body = ctx.body_text().await;
        let ok = body.find(&alpha_title).is_some()
            && body.find(&alpha_title).expect("alpha") < body.find(&gamma_title).expect("gamma");
        if ok {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "title desc 排序未收敛; body: {:?}",
            body.chars().take(300).collect::<String>()
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let removed: bool = ctx
        .page
        .evaluate(
            "(() => { const b = Array.from(document.querySelectorAll('button[type=\"button\"]')).find(el => el.textContent.trim() === '×'); if (!b) return false; b.click(); return true; })()",
        )
        .await
        .expect("evaluate remove sort")
        .into_value()
        .expect("remove sort value");
    assert!(removed, "移除按钮 × 未找到");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let url = ctx.page.url().await.expect("page url").unwrap_or_default();
        if !url.contains("sort=") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "sort= 从未从 URL 移除; url: {url}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    ctx.wait_for_text(&alpha_title, 10).await;
}

#[tokio::test]
async fn ui_datetime_from_to_bounds() {
    let _serial = BROWSER_SERIAL.lock().await;
    let ctx = EndToEndBrowserContext::new().await;
    let session = ctx.login_via_api(&unique_email("sdt")).await;
    ctx.set_session_token(&session).await;
    let marker = format!("Dt{}", uuid::Uuid::now_v7());
    let first_title = format!("{marker} first");
    let second_title = format!("{marker} second");
    ctx.create_article(&session, &first_title, "s", "#d", "1.0.0", "n")
        .await;
    ctx.create_article(&session, &second_title, "s", "#d", "1.0.0", "n")
        .await;
    let base = format!("{}/public/article/search", ctx.base_url);

    ctx.page.goto(&base).await.expect("goto search");
    ctx.wait_for_text(&first_title, 10).await;
    ctx.wait_for_text(&second_title, 10).await;

    let set_to_past: bool = ctx
        .page
        .evaluate(
            "(() => { const inputs = document.querySelectorAll('input[type=datetime-local]'); if (inputs.length < 2) return false; const to = inputs[1]; to.value = '1990-01-01T00:00'; to.dispatchEvent(new Event('change', { bubbles: true })); return true; })()",
        )
        .await
        .expect("evaluate set to")
        .into_value()
        .expect("set to value");
    assert!(set_to_past, "to datetime-local 输入框未找到");
    wait_for_url_fragment(&ctx, "to=").await;
    ctx.wait_for_text("none", 10).await;

    let set_from_past: bool = ctx
        .page
        .evaluate(
            "(() => { const inputs = document.querySelectorAll('input[type=datetime-local]'); if (inputs.length < 2) return false; inputs[1].value = ''; inputs[1].dispatchEvent(new Event('change', { bubbles: true })); inputs[0].value = '2000-01-01T00:00'; inputs[0].dispatchEvent(new Event('change', { bubbles: true })); return true; })()",
        )
        .await
        .expect("evaluate set from")
        .into_value()
        .expect("set from value");
    assert!(set_from_past, "from datetime-local 输入框未找到");
    wait_for_url_fragment(&ctx, "from=").await;
    ctx.wait_for_text(&first_title, 10).await;
    ctx.wait_for_text(&second_title, 10).await;
}
