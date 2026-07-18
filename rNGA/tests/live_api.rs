use rnga::{ForumIdKind, NGAClient, Result};

#[tokio::test]
#[ignore = "requires network access to NGA and valid RNGA_TEST_TOKEN/RNGA_TEST_UID"]
async fn live_forum_list() -> Result<()> {
    let token = std::env::var("RNGA_TEST_TOKEN").expect("RNGA_TEST_TOKEN");
    let uid = std::env::var("RNGA_TEST_UID").expect("RNGA_TEST_UID");

    let client = NGAClient::builder().auth(token, uid).build()?;
    let categories = client.forums().list().await?;
    assert!(!categories.is_empty());

    let topics = client
        .topics()
        .list(ForumIdKind::fid("7"))
        .send()
        .await?;
    assert!(!topics.topics.is_empty() || topics.total_pages >= 1);

    Ok(())
}
