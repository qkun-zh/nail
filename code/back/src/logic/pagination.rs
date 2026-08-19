use crate::logic::error::LogicError;

pub const MAX_PAGE_SIZE: u64 = 200;
pub const MAX_PAGE: u64 = 10_000;

pub fn page_offset(page: u64, limit: u64) -> u64 {
    page.saturating_sub(1).saturating_mul(limit)
}

pub fn paginate<T>(items: Vec<T>, page: u64, limit: u64) -> (Vec<T>, bool) {
    let total = items.len() as u64;
    let items = items
        .into_iter()
        .skip(usize::try_from(page_offset(page, limit)).unwrap_or(usize::MAX))
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .collect();
    let has_next = page < total.div_ceil(limit);
    (items, has_next)
}

pub fn clamp_page_limit(
    page: Option<u64>,
    limit: Option<u64>,
    default_limit: u64,
    max_pages: u64,
) -> Result<(u64, u64), LogicError> {
    let limit = limit.unwrap_or(default_limit).clamp(1, MAX_PAGE_SIZE);
    let page = match page {
        Some(page) if page > max_pages => {
            return Err(LogicError::bad_request("page exceeds max search pages"));
        }
        Some(page) => page,
        None => 1,
    }
    .clamp(1, MAX_PAGE);
    Ok((page, limit))
}
