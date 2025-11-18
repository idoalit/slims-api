pub mod biblios;
pub mod items;
pub mod loans;
pub mod lookups;
pub mod members;
pub mod visitors;
pub mod files;
pub mod settings;
pub mod contents;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const DEFAULT_PAGE: u32 = 1;
const DEFAULT_PER_PAGE: u32 = 20;
const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Pagination {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl Pagination {
    fn resolved(&self) -> (u32, u32) {
        let page = self.page.unwrap_or(DEFAULT_PAGE).max(1);
        let per_page = self
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        (page, per_page)
    }

    pub fn limit_offset(&self) -> (i64, i64, u32, u32) {
        let (page, per_page) = self.resolved();
        let offset = ((page - 1) * per_page) as i64;
        (per_page as i64, offset, page, per_page)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub include: Option<String>,
}

impl ListParams {
    pub fn pagination(&self) -> Pagination {
        Pagination {
            page: self.page,
            per_page: self.per_page,
        }
    }

    pub fn includes(&self) -> HashSet<String> {
        parse_include(self.include.clone())
    }
}

pub fn parse_include(raw: Option<String>) -> HashSet<String> {
    raw.map(|s| {
        s.split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_lowercase())
            })
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Debug, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
}
