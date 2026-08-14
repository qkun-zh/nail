pub const MAX_PAGE_SIZE: u64 = 200;
pub const MAX_PAGE: u64 = 10_000;

pub fn clamp_page_limit(page: Option<u64>, limit: Option<u64>, default_limit: u64) -> (u64, u64) {
    let limit = limit.unwrap_or(default_limit).clamp(1, MAX_PAGE_SIZE);
    let page = page.unwrap_or(1).clamp(1, MAX_PAGE);
    (page, limit)
}
