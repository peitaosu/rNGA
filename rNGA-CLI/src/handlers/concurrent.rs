//! Concurrent page fetching helpers.

use anyhow::Context;
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use tokio::sync::Semaphore;

use super::meta::PageWarning;

pub async fn fetch_pages_concurrent<F, Fut, T>(
    start_page: u32,
    num_pages: u32,
    concurrency: usize,
    fetch_page: F,
) -> (Vec<T>, Vec<PageWarning>)
where
    F: Fn(u32) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<T, anyhow::Error>> + Send,
    T: Send,
{
    if num_pages <= 1 {
        return (Vec::new(), Vec::new());
    }

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let fetch_page = Arc::new(fetch_page);

    let fetch_results: Vec<(u32, Result<T, anyhow::Error>)> = stream::iter((start_page + 1)..=(start_page + num_pages - 1))
        .map(|page| {
            let semaphore = semaphore.clone();
            let fetch_page = fetch_page.clone();
            async move {
                let permit = semaphore.acquire().await;
                let result = match permit {
                    Ok(_permit) => fetch_page(page).await,
                    Err(error) => Err(error).context("acquiring fetch semaphore"),
                };
                (page, result)
            }
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;

    let mut items = Vec::new();
    let mut warnings = Vec::new();

    let mut sorted_results = fetch_results;
    sorted_results.sort_by_key(|(page, _)| *page);

    for (page, result) in sorted_results {
        match result {
            Ok(item) => items.push(item),
            Err(error) => warnings.push(PageWarning {
                page,
                error: error.to_string(),
            }),
        }
    }

    (items, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_fetch_pages_concurrent_collects_successes_and_warnings() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_for_fetch = attempts.clone();

        let (items, warnings) = fetch_pages_concurrent(1, 4, 2, move |page| {
            let attempts = attempts_for_fetch.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                if page == 3 {
                    Err(anyhow::anyhow!("page 3 failed"))
                } else {
                    Ok(page)
                }
            }
        })
        .await;

        assert_eq!(items, vec![2, 4]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].page, 3);
        assert!(attempts.load(Ordering::SeqCst) >= 3);
    }
}
