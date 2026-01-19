//! HTTP client configuration and request execution.

use crate::error::{Error, Result};
use encoding_rs::GB18030;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::time::Duration;
use url::Url;

/// Decode bytes as GB18030.
fn decode_gb18030(bytes: &[u8]) -> String {
    let (text, _, _) = GB18030.decode(bytes);
    text.into_owned()
}

/// Default NGA API base URL.
pub const DEFAULT_BASE_URL: &str = "https://nga.178.com/";

/// Forum icon CDN path.
pub const FORUM_ICON_PATH: &str = "http://img4.ngacn.cc/ngabbs/nga_classic/f/app/";

/// User agents for different platforms.
pub mod user_agents {
    pub const APPLE: &str = "NGA_skull/7.3.1(iPhone17,1;iOS 26.0)";
    pub const ANDROID: &str = "Nga_Official/80024(Android12)";
    pub const DESKTOP: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36";
    pub const WINDOWS_PHONE: &str = "NGA_WP_JW/(;WINDOWS)";
}

/// Device type for requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Device {
    #[default]
    Apple,
    Android,
    Desktop,
    WindowsPhone,
}

impl Device {
    /// Get the user agent string for this device.
    pub fn user_agent(&self) -> &'static str {
        match self {
            Device::Apple => user_agents::APPLE,
            Device::Android => user_agents::ANDROID,
            Device::Desktop => user_agents::DESKTOP,
            Device::WindowsPhone => user_agents::WINDOWS_PHONE,
        }
    }
}

/// HTTP client configuration.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Base URL for API requests.
    pub base_url: String,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Read timeout.
    pub read_timeout: Duration,
    /// Device type for User-Agent.
    pub device: Device,
    /// Custom user agent.
    pub custom_user_agent: Option<String>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_owned(),
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(20),
            device: Device::default(),
            custom_user_agent: None,
        }
    }
}

impl HttpConfig {
    /// Get the user agent to use for a specific API endpoint.
    pub fn user_agent_for(&self, api: &str) -> &str {
        if let Some(ref ua) = self.custom_user_agent {
            return ua;
        }
        if api == "read.php" {
            return user_agents::ANDROID;
        }
        self.device.user_agent()
    }

    /// Resolve a relative API path to a full URL.
    pub fn resolve_url(&self, api: &str) -> Result<Url> {
        if api.starts_with("http://") || api.starts_with("https://") {
            return Url::parse(api).map_err(Error::Url);
        }

        Url::parse(&self.base_url)
            .and_then(|b| b.join(api))
            .map_err(Error::Url)
    }
}

/// Build a reqwest client with the given configuration.
pub fn build_client(config: &HttpConfig) -> Result<Client> {
    Client::builder()
        .https_only(false)
        .connect_timeout(config.connect_timeout)
        .read_timeout(config.read_timeout)
        .gzip(true)
        .build()
        .map_err(Error::Network)
}

/// Query parameter strategies for NGA API.
#[derive(Debug, Clone, Copy)]
pub enum ResponseFormat {
    /// XML format: `lite=xml`
    Xml,
    /// Compact XML: `__output=10`
    CompactXml,
    /// JSON format: `__output=8`
    #[allow(dead_code)]
    Json,
}

impl ResponseFormat {
    /// Get the query parameter for this format.
    pub fn query_param(&self) -> (&'static str, &'static str) {
        match self {
            ResponseFormat::Xml => ("lite", "xml"),
            ResponseFormat::CompactXml => ("__output", "10"),
            ResponseFormat::Json => ("__output", "8"),
        }
    }
}

/// HTTP request executor.
pub struct HttpExecutor<'a> {
    client: &'a Client,
    config: &'a HttpConfig,
}

impl<'a> HttpExecutor<'a> {
    /// Create a new executor.
    pub fn new(client: &'a Client, config: &'a HttpConfig) -> Self {
        Self { client, config }
    }

    /// Build a request with common headers.
    fn build_request(&self, method: Method, url: Url, api: &str) -> RequestBuilder {
        let ua = self.config.user_agent_for(api);
        let referer = url.to_string();

        self.client
            .request(method, url)
            .header("User-Agent", ua)
            .header("X-User-Agent", ua)
            .header("Referer", referer)
    }

    /// Execute a POST request with form data and return the response text.
    pub async fn post_form(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
    ) -> Result<String> {
        self.post_form_with_format(api, query, form, auth, ResponseFormat::Xml)
            .await
    }

    /// Execute a POST request with specific response format.
    pub async fn post_form_with_format(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
        format: ResponseFormat,
    ) -> Result<String> {
        let url = self.config.resolve_url(api)?;

        let mut full_query: Vec<(&str, &str)> = query
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .copied()
            .collect();
        full_query.push(format.query_param());
        full_query.push(("__inchst", "UTF8"));

        let mut full_form: Vec<(&str, &str)> = form.to_vec();
        if let Some((token, uid)) = auth {
            full_form.push(("access_token", token));
            full_form.push(("access_uid", uid));
        } else {
            full_form.push(("access_token", ""));
            full_form.push(("access_uid", ""));
        }

        let request = self
            .build_request(Method::POST, url, api)
            .query(&full_query)
            .form(&full_form);

        let response = request.send().await.map_err(Error::Network)?;
        self.handle_response(response).await
    }

    /// Execute a POST request with XML response and automatic retry.
    pub async fn post_form_xml(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
    ) -> Result<String> {
        let text = self
            .post_form_with_format(api, query, form, auth, ResponseFormat::Xml)
            .await?;
        if !text.is_empty() && sxd_document::parser::parse(&text).is_ok() {
            return Ok(text);
        }

        let text = self
            .post_form_with_format(api, query, form, auth, ResponseFormat::CompactXml)
            .await?;
        if !text.is_empty() && sxd_document::parser::parse(&text).is_ok() {
            return Ok(text);
        }

        if auth.is_some() {
            let text = self
                .post_form_with_format(api, query, form, None, ResponseFormat::Xml)
                .await?;
            if !text.is_empty() && sxd_document::parser::parse(&text).is_ok() {
                return Ok(text);
            }
        }

        Err(Error::Xml(
            "All retry attempts returned malformed XML".into(),
        ))
    }

    /// Handle response, decoding with proper charset.
    async fn handle_response(&self, response: Response) -> Result<String> {
        let status = response.status();

        let bytes = response.bytes().await.map_err(Error::Network)?;

        let text = decode_gb18030(&bytes);

        if text.is_empty() && !status.is_success() {
            return Err(Error::nga(
                status.as_u16().to_string(),
                status.canonical_reason().unwrap_or("Unknown error"),
            ));
        }

        Ok(text)
    }

    /// Execute a JSON request.
    #[allow(dead_code)]
    pub async fn post_json(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
    ) -> Result<serde_json::Value> {
        let text = self
            .post_form_with_format(api, query, form, auth, ResponseFormat::Json)
            .await?;

        parse_json_response(&text)
    }
}

/// Parse JSON response from NGA.
#[allow(dead_code)]
fn parse_json_response(text: &str) -> Result<serde_json::Value> {
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str(text))
        .map_err(Error::Json)?;

    if let Some(data) = value.get_mut("data") {
        Ok(data.take())
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url() {
        let config = HttpConfig::default();

        let url = config.resolve_url("thread.php").unwrap();
        assert!(url.as_str().contains("nga.178.com"));
        assert!(url.as_str().ends_with("thread.php"));
    }
}
