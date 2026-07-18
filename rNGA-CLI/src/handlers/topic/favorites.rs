use anyhow::Context;
use rnga::models::FavoriteTopicOp;
use rnga::NGAClient;

use super::types::{CliFavoriteTopicsResult, FavoriteModifyResult, FolderInfo};

pub async fn list_folders(client: &NGAClient) -> anyhow::Result<Vec<FolderInfo>> {
    let folders = client
        .topics()
        .favorite_folders()
        .await
        .context("listing favorite folders")?;
    Ok(folders.iter().map(FolderInfo::from).collect())
}

pub async fn list_favorites(
    client: &NGAClient,
    folder: Option<&str>,
    page: u32,
) -> anyhow::Result<CliFavoriteTopicsResult> {
    let mut builder = client.topics().favorites().page(page);
    if let Some(folder_id) = folder {
        builder = builder.folder(folder_id.to_string());
    }

    let result = builder.send().await.context("listing favorite topics")?;

    Ok(CliFavoriteTopicsResult {
        folder: folder.map(|s| s.to_string()),
        page,
        total_pages: result.total_pages,
        topics: result.topics.iter().map(super::types::TopicSummary::from).collect(),
    })
}

pub async fn add_favorite(
    client: &NGAClient,
    topic_id: &str,
    folder: Option<&str>,
) -> anyhow::Result<FavoriteModifyResult> {
    let folder_id = folder.unwrap_or("");
    client
        .topics()
        .modify_favorite(topic_id, folder_id, FavoriteTopicOp::Add)
        .await
        .context("adding topic favorite")?;

    Ok(FavoriteModifyResult {
        topic_id: topic_id.to_string(),
        action: "added".to_string(),
    })
}

pub async fn remove_favorite(
    client: &NGAClient,
    topic_id: &str,
    folder: Option<&str>,
) -> anyhow::Result<FavoriteModifyResult> {
    let folder_id = folder.unwrap_or("");
    client
        .topics()
        .modify_favorite(topic_id, folder_id, FavoriteTopicOp::Remove)
        .await
        .context("removing topic favorite")?;

    Ok(FavoriteModifyResult {
        topic_id: topic_id.to_string(),
        action: "removed".to_string(),
    })
}
