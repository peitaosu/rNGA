//! HTTP client configuration and request execution.

use crate::error::{Error, Result};
use encoding_rs::GB18030;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use url::Url;

fn decode_gb18030(bytes: &[u8]) -> String {
    let (text, _, _) = GB18030.decode(bytes);
    text.into_owned()
}

pub const DEFAULT_BASE_URL: &str = "https://ngabbs.com/";

pub const FORUM_ICON_PATH: &str = "http://img4.ngacn.cc/ngabbs/nga_classic/f/app/";

const MAX_HTTP_ATTEMPTS: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 150;

pub mod user_agents {
    pub const APPLE: &str = "NGA_skull/7.3.1(iPhone17,1;iOS 26.0)";
    pub const ANDROID: &str = "Nga_Official/80024(Android12)";
    pub const DESKTOP: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.64 Safari/537.36";
    pub const WINDOWS_PHONE: &str = "NGA_WP_JW/(;WINDOWS)";
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Device {
    #[default]
    Apple,
    Android,
    Desktop,
    WindowsPhone,
}

impl Device {
    pub fn user_agent(&self) -> &'static str {
        match self {
            Device::Apple => user_agents::APPLE,
            Device::Android => user_agents::ANDROID,
            Device::Desktop => user_agents::DESKTOP,
            Device::WindowsPhone => user_agents::WINDOWS_PHONE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub base_url: String,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub device: Device,
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
    pub fn user_agent_for(&self, api: &str) -> &str {
        if let Some(ref ua) = self.custom_user_agent {
            return ua;
        }
        if api == "read.php" {
            return user_agents::ANDROID;
        }
        self.device.user_agent()
    }

    pub fn resolve_url(&self, api: &str) -> Result<Url> {
        if api.starts_with("http://") || api.starts_with("https://") {
            return Url::parse(api).map_err(Error::Url);
        }

        Url::parse(&self.base_url)
            .and_then(|b| b.join(api))
            .map_err(Error::Url)
    }
}

pub fn build_client(config: &HttpConfig) -> Result<Client> {
    Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(config.connect_timeout)
        .read_timeout(config.read_timeout)
        .gzip(true)
        .build()
        .map_err(Error::Network)
}

#[derive(Debug, Clone, Copy)]
pub enum ResponseFormat {
    WebXml,
    AppJson,
}

impl ResponseFormat {
    pub fn query_param(&self) -> (&'static str, &'static str) {
        match self {
            ResponseFormat::WebXml => ("lite", "xml"),
            ResponseFormat::AppJson => ("__output", "8"),
        }
    }
}

fn cookie_header(auth: Option<(&str, &str)>) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut cookie = format!("guestJs={timestamp};");
    if let Some((token, uid)) = auth {
        if !token.is_empty() && !uid.is_empty() {
            cookie.push_str(&format!(" ngaPassportUid={uid}; ngaPassportCid={token};"));
        }
    }
    cookie
}

pub struct HttpExecutor<'a> {
    client: &'a Client,
    config: &'a HttpConfig,
}

impl<'a> HttpExecutor<'a> {
    pub fn new(client: &'a Client, config: &'a HttpConfig) -> Self {
        Self { client, config }
    }

    fn build_request(
        &self,
        method: Method,
        url: Url,
        api: &str,
        auth: Option<(&str, &str)>,
    ) -> RequestBuilder {
        let ua = self.config.user_agent_for(api);
        let referer = url.to_string();

        self.client
            .request(method, url)
            .header("User-Agent", ua)
            .header("X-User-Agent", ua)
            .header("Referer", referer)
            .header("Cookie", cookie_header(auth))
    }

    pub async fn post_form_with_format(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
        format: ResponseFormat,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..MAX_HTTP_ATTEMPTS {
            match self
                .execute_post_form(api, query, form, auth, format)
                .await
            {
                Ok(text) => return Ok(text),
                Err(error) if error.is_retryable() && attempt + 1 < MAX_HTTP_ATTEMPTS => {
                    last_error = Some(error);
                    sleep(Duration::from_millis(
                        RETRY_BASE_DELAY_MS * (attempt as u64 + 1),
                    ))
                    .await;
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Internal("HTTP request failed".into())))
    }

    async fn execute_post_form(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
        format: ResponseFormat,
    ) -> Result<String> {
        let url = self.config.resolve_url(api)?;

        let full_query: Vec<(&str, &str)> = query
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .copied()
            .collect();

        let mut full_form: Vec<(&str, &str)> = form.to_vec();
        full_form.push(format.query_param());
        full_form.push(("__inchst", "UTF8"));
        if let Some((token, uid)) = auth {
            full_form.push(("access_token", token));
            full_form.push(("access_uid", uid));
        } else {
            full_form.push(("access_token", ""));
            full_form.push(("access_uid", ""));
        }

        let request = self
            .build_request(Method::POST, url, api, auth)
            .query(&full_query)
            .form(&full_form);

        let response = request.send().await.map_err(Error::Network)?;
        self.handle_response(response).await
    }

    async fn handle_response(&self, response: Response) -> Result<String> {
        let status = response.status();
        let bytes = response.bytes().await.map_err(Error::Network)?;
        let text = decode_gb18030(&bytes);

        if text.is_empty() && !status.is_success() {
            let code = status.as_u16().to_string();
            let message = status.canonical_reason().unwrap_or("Unknown error");
            return Err(Error::nga(code, message));
        }

        Ok(text)
    }

    pub async fn post_form_xml(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
    ) -> Result<String> {
        let text = self
            .post_form_with_format(api, query, form, auth, ResponseFormat::WebXml)
            .await?;
        if is_valid_xml(&text) {
            return Ok(text);
        }

        Err(Error::Xml("response is not valid XML".into()))
    }

    pub async fn post_json(
        &self,
        api: &str,
        query: &[(&str, &str)],
        form: &[(&str, &str)],
        auth: Option<(&str, &str)>,
    ) -> Result<serde_json::Value> {
        let text = self
            .post_form_with_format(api, query, form, auth, ResponseFormat::AppJson)
            .await?;

        parse_json_response(&text)
    }
}

fn is_valid_xml(text: &str) -> bool {
    !text.is_empty() && sxd_document::parser::parse(text).is_ok()
}

fn parse_json_response(text: &str) -> Result<serde_json::Value> {
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(Error::Json)?;

    if let Some(code) = value.get("code").and_then(|code| code.as_i64()) {
        if code != 0 {
            let message = value
                .get("msg")
                .or_else(|| value.get("message"))
                .and_then(|msg| msg.as_str())
                .unwrap_or("unknown error");
            return Err(Error::nga(code.to_string(), message));
        }
    }

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
        assert!(url.as_str().contains("ngabbs.com"));
        assert!(url.as_str().ends_with("thread.php"));
    }

    #[test]
    fn test_default_base_url_uses_canonical_host() {
        assert_eq!(DEFAULT_BASE_URL, "https://ngabbs.com/");
    }

    #[test]
    fn test_cookie_header_guest_only() {
        let cookie = cookie_header(None);
        assert!(cookie.starts_with("guestJs="));
        assert!(!cookie.contains("ngaPassportUid"));
    }

    #[test]
    fn test_cookie_header_authenticated() {
        let cookie = cookie_header(Some(("token123", "uid456")));
        assert!(cookie.contains("guestJs="));
        assert!(cookie.contains("ngaPassportUid=uid456"));
        assert!(cookie.contains("ngaPassportCid=token123"));
    }

    #[test]
    fn test_is_valid_xml() {
        assert!(is_valid_xml(r#"<?xml version="1.0"?><root/>"#));
        assert!(!is_valid_xml(""));
        assert!(!is_valid_xml("not xml"));
    }

    #[test]
    fn test_parse_json_response_rejects_api_error() {
        let error = parse_json_response(r#"{"code":1,"msg":"auth required"}"#).unwrap_err();
        assert!(matches!(error, Error::NGAApi { code, .. } if code == "1"));
    }

    #[test]
    fn test_parse_json_response_unwraps_data() {
        let value = parse_json_response(r#"{"code":0,"data":{"result":[]}}"#).unwrap();
        assert!(value.get("result").is_some());
    }
}
