const MAX_PAGE_SIZE: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationState {
    pub page: u64,
    pub previous_page: Option<u64>,
    pub next_page: Option<u64>,
}

pub fn pagination_state(page: u64, server_has_next: bool) -> PaginationState {
    let page = page.max(1);
    PaginationState {
        page,
        previous_page: (page > 1).then(|| page - 1),
        next_page: server_has_next.then(|| page + 1),
    }
}

pub fn clamp_page_size(limit: u64, fallback: u64) -> u64 {
    if limit == 0 {
        fallback
    } else {
        limit.clamp(1, MAX_PAGE_SIZE)
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/pagination/tests.rs"]
mod tests;
