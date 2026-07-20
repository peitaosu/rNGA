const AVAILABLE_LOCALES: &[&str] = &["en", "zh-CN"];

pub fn resolve(explicit: Option<&str>) -> String {
    let candidate = explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(detect_system_locale);

    normalize_and_validate(&candidate)
}

fn detect_system_locale() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(var) {
            if let Some(locale) = map_env_locale(&value) {
                return locale;
            }
        }
    }
    "zh-CN".to_string()
}

fn map_env_locale(value: &str) -> Option<String> {
    let tag = value.split('.').next()?.split(':').next()?.trim();
    if tag.is_empty() || tag == "C" || tag == "POSIX" {
        return None;
    }

    let normalized = tag.replace('_', "-");
    if normalized.starts_with("zh") {
        Some("zh-CN".to_string())
    } else if normalized.starts_with("en") {
        Some("en".to_string())
    } else {
        Some(normalized)
    }
}

fn normalize_and_validate(locale: &str) -> String {
    let normalized = if locale.starts_with("zh") {
        "zh-CN".to_string()
    } else if locale.starts_with("en") {
        "en".to_string()
    } else {
        locale.to_string()
    };

    if AVAILABLE_LOCALES.contains(&normalized.as_str()) {
        normalized
    } else {
        "zh-CN".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_explicit_locale_is_used() {
        assert_eq!(resolve(Some("en")), "en");
        assert_eq!(resolve(Some("zh-CN")), "zh-CN");
    }

    #[test]
    fn test_unsupported_explicit_locale_falls_back_to_zh_cn() {
        assert_eq!(resolve(Some("fr")), "zh-CN");
    }

    #[test]
    fn test_detects_zh_locale_from_env() {
        let _lock = env_lock().lock().unwrap();
        let _lang = EnvVarGuard::set("LANG", "zh_CN.UTF-8");
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");

        assert_eq!(resolve(None), "zh-CN");
    }

    #[test]
    fn test_detects_en_locale_from_env() {
        let _lock = env_lock().lock().unwrap();
        let _lang = EnvVarGuard::set("LANG", "en_US.UTF-8");
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");

        assert_eq!(resolve(None), "en");
    }

    #[test]
    fn test_unsupported_env_locale_falls_back_to_zh_cn() {
        let _lock = env_lock().lock().unwrap();
        let _lang = EnvVarGuard::set("LANG", "fr_FR.UTF-8");
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");

        assert_eq!(resolve(None), "zh-CN");
    }
}
