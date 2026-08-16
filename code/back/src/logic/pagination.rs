use crate::logic::error::LogicError;

pub const MAX_PAGE_SIZE: u64 = 200;
pub const MAX_PAGE: u64 = 10_000;

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
