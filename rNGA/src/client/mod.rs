//! HTTP client and configuration.

mod auth;
mod http;

pub use auth::AuthInfo;
pub use http::{Device, HttpConfig, FORUM_ICON_PATH};

use crate::api::{ForumApi, MessageApi, NotificationApi, PostApi, TopicApi, UserApi};
use crate::cache::CacheStorage;
use crate::error::{Error, Result};
use http::{build_client, HttpExecutor};
use std::sync::Arc;
use std::time::Duration;

/// Builder for creating NGAClient.
pub struct NGAClientBuilder {
    auth: Option<AuthInfo>,
    http_config: HttpConfig,
    cache: Option<Arc<dyn CacheStorage>>,
}

impl std::fmt::Debug for NGAClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NGAClientBuilder")
            .field("auth", &self.auth.as_ref().map(|a| &a.uid))
            .field("http_config", &self.http_config)
            .field("cache", &self.cache.as_ref().map(|_| "..."))
            .finish()
    }
}

impl Default for NGAClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NGAClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            auth: None,
            http_config: HttpConfig::default(),
            cache: None,
        }
    }

    /// Set authentication.
    pub fn auth(mut self, token: impl Into<String>, uid: impl Into<String>) -> Self {
        self.auth = Some(AuthInfo::new(token, uid));
        self
    }

    /// Set authentication from AuthInfo.
    pub fn with_auth(mut self, auth: AuthInfo) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.http_config.base_url = url.into();
        self
    }

    /// Set device type.
    pub fn device(mut self, device: Device) -> Self {
        self.http_config.device = device;
        self
    }

    /// Set custom user agent.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.http_config.custom_user_agent = Some(ua.into());
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.http_config.connect_timeout = timeout;
        self
    }

    /// Set read timeout.
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.http_config.read_timeout = timeout;
        self
    }

    /// Set cache storage.
    pub fn cache(mut self, storage: Arc<dyn CacheStorage>) -> Self {
        self.cache = Some(storage);
        self
    }

    /// Build NGAClient.
    pub fn build(self) -> Result<NGAClient> {
        let http_client = build_client(&self.http_config)?;

        Ok(NGAClient {
            inner: Arc::new(NGAClientInner {
                http: http_client,
                config: self.http_config,
                auth: self.auth,
                cache: self.cache,
            }),
        })
    }
}

/// Internal client state.
pub(crate) struct NGAClientInner {
    pub http: reqwest::Client,
    pub config: HttpConfig,
    pub auth: Option<AuthInfo>,
    /// Cache storage for API responses
    #[allow(dead_code)]
    pub cache: Option<Arc<dyn CacheStorage>>,
}

impl NGAClientInner {
    /// Get auth info or error.
    pub fn require_auth(&self) -> Result<&AuthInfo> {
        self.auth.as_ref().ok_or(Error::AuthRequired)
    }

    /// Get auth as tuple or error.
    pub fn auth_tuple(&self) -> Result<(&str, &str)> {
        let auth = self.require_auth()?;
        Ok((&auth.token, &auth.uid))
    }

    /// Get optional auth as tuple.
    pub fn auth_tuple_opt(&self) -> Option<(&str, &str)> {
        self.auth
            .as_ref()
            .map(|a| (a.token.as_str(), a.uid.as_str()))
    }

    /// Create HTTP executor.
    pub fn executor(&self) -> HttpExecutor<'_> {
        HttpExecutor::new(&self.http, &self.config)
    }

    /// Execute authenticated POST request.
    pub async fn post_authed(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
    ) -> Result<String> {
        let auth = self.auth_tuple()?;
        self.executor()
            .post_form(api, query, form, Some(auth))
            .await
    }

    /// Execute a POST request.
    pub async fn post(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
    ) -> Result<String> {
        let auth = self.auth_tuple_opt();
        self.executor().post_form_xml(api, query, form, auth).await
    }

    /// Execute a JSON POST request.
    #[allow(dead_code)]
    pub async fn post_json(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let auth = self.auth_tuple_opt();
        self.executor().post_json(api, query, form, auth).await
    }
}

/// NGA client for interacting with the forum.
#[derive(Clone)]
pub struct NGAClient {
    pub(crate) inner: Arc<NGAClientInner>,
}

impl NGAClient {
    /// Create a new client builder.
    pub fn builder() -> NGAClientBuilder {
        NGAClientBuilder::new()
    }

    /// Get the forum API.
    pub fn forums(&self) -> ForumApi {
        ForumApi::new(self.inner.clone())
    }

    /// Get the topic API.
    pub fn topics(&self) -> TopicApi {
        TopicApi::new(self.inner.clone())
    }

    /// Get the post API.
    pub fn posts(&self) -> PostApi {
        PostApi::new(self.inner.clone())
    }

    /// Get the user API.
    pub fn users(&self) -> UserApi {
        UserApi::new(self.inner.clone())
    }

    /// Get the notification API.
    pub fn notifications(&self) -> NotificationApi {
        NotificationApi::new(self.inner.clone())
    }

    /// Get the message API.
    pub fn messages(&self) -> MessageApi {
        MessageApi::new(self.inner.clone())
    }

    /// Check if the client is authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.inner.auth.is_some()
    }

    /// Get the current authentication info.
    pub fn auth_info(&self) -> Option<&AuthInfo> {
        self.inner.auth.as_ref()
    }

    /// Get the current user ID if authenticated.
    pub fn current_uid(&self) -> Option<&str> {
        self.inner.auth.as_ref().map(|a| a.uid.as_str())
    }
}

impl std::fmt::Debug for NGAClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NGAClient")
            .field("authenticated", &self.is_authenticated())
            .field("base_url", &self.inner.config.base_url)
            .finish()
    }
}
