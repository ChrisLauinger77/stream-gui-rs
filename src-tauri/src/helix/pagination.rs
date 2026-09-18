use crate::{
    domain::{AppError, ErrorCode, Result},
    twitch_http::error,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Pagination {
    pub cursor: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: Pagination,
    pub total: Option<u64>,
}
impl<T> Page<T> {
    pub fn next(&self, size: u8) -> Result<Option<PageRequest>> {
        self.pagination
            .cursor
            .as_deref()
            .filter(|cursor| !cursor.is_empty())
            .map(|cursor| PageRequest::after(size, cursor))
            .transpose()
    }
}
#[derive(Clone, Debug)]
pub struct PageRequest {
    first: u8,
    cursor: Option<(bool, String)>,
}
impl Default for PageRequest {
    fn default() -> Self {
        Self {
            first: 20,
            cursor: None,
        }
    }
}
impl PageRequest {
    pub fn first(size: u8) -> Result<Self> {
        if !(1..=100).contains(&size) {
            return Err(error(ErrorCode::InvalidInput));
        }
        Ok(Self {
            first: size,
            cursor: None,
        })
    }
    pub fn after(size: u8, cursor: &str) -> Result<Self> {
        Self::cursor(size, cursor, false)
    }
    pub fn before(size: u8, cursor: &str) -> Result<Self> {
        Self::cursor(size, cursor, true)
    }
    fn cursor(size: u8, cursor: &str, before: bool) -> Result<Self> {
        let mut request = Self::first(size)?;
        if cursor.is_empty() || cursor.len() > 2048 {
            return Err(error(ErrorCode::InvalidInput));
        }
        request.cursor = Some((before, cursor.into()));
        Ok(request)
    }
    pub(crate) fn query(&self, backwards: bool) -> Result<Vec<(String, String)>> {
        let mut query = vec![("first".into(), self.first.to_string())];
        if let Some((before, cursor)) = &self.cursor {
            if *before && !backwards {
                return Err(error(ErrorCode::InvalidInput));
            }
            query.push((
                if *before { "before" } else { "after" }.into(),
                cursor.clone(),
            ));
        }
        Ok(query)
    }
}

/// Successful chunks and the IDs whose chunks failed remain distinguishable.
/// Missing IDs in a successful response (offline users, deleted metadata) are
/// reported separately, never confused with a failed request.
#[derive(Debug)]
pub struct BatchResult<T> {
    pub data: Vec<T>,
    pub missing_ids: Vec<String>,
    pub stale_ids: Vec<String>,
    pub incomplete_ids: Vec<String>,
    pub failures: Vec<BatchFailure>,
}
#[derive(Debug)]
pub struct BatchFailure {
    pub ids: Vec<String>,
    pub error: AppError,
}
impl<T> BatchResult<T> {
    pub fn complete(&self) -> bool {
        self.failures.is_empty() && self.incomplete_ids.is_empty()
    }
}

pub fn id_batches(ids: &[String]) -> Result<Vec<Vec<String>>> {
    if ids.len() > 1000 {
        return Err(error(ErrorCode::InvalidInput));
    }
    let mut seen = HashSet::new();
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut bytes = 0;
    for id in ids {
        if id.is_empty() || id.len() > 128 {
            return Err(error(ErrorCode::InvalidInput));
        }
        if !seen.insert(id) {
            continue;
        }
        let encoded_len = url::form_urlencoded::byte_serialize(id.as_bytes())
            .map(str::len)
            .sum::<usize>()
            + 20;
        if batch.len() == 100 || bytes + encoded_len > 6000 {
            batches.push(std::mem::take(&mut batch));
            bytes = 0;
        }
        bytes += encoded_len;
        batch.push(id.clone());
    }
    if !batch.is_empty() {
        batches.push(batch);
    }
    Ok(batches)
}
