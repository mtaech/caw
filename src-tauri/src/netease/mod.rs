//! NetEase Cloud Music module for Caw.
//!
//! Optional integration with the NetEase Cloud Music Open Platform. When
//! disabled in Settings this module stays inactive and sends no requests.
//!
//! Protocol notes live in docs/netease-openapi.md.

pub mod client;
pub mod sign;
pub mod types;

pub use client::NeteaseClient;
pub use types::*;

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// Read a trimmed non-empty value from the process environment.
fn env_var(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Load a .env file from the working directory or the project root into the
/// process environment. Dev convenience: it lets credentials live outside the
/// Settings UI. Existing environment variables always win.
///
/// Deliberately minimal (KEY=VALUE, # comments, optional quotes) to avoid
/// pulling in a dependency for a dev-only affordance.
pub fn load_dotenv() {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
        // `cargo tauri dev` runs the binary from src-tauri/.
        if let Some(parent) = cwd.parent() {
     candidates.push(parent.join(".env"));
        }
    }

    for path in candidates {
let Ok(content) = std::fs::read_to_string(&path) else {
          continue;
   };

    for line in content.lines() {
    let line = line.trim();
   if line.is_empty() || line.starts_with('#') {
       continue;
     }
   let Some((key, value)) = line.split_once('=') else {
   continue;
      };
     let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
     if key.is_empty() || value.is_empty() {
      continue;
      }
      if std::env::var(key).is_err() {
         unsafe { std::env::set_var(key, value) };
 }
        }

   crate::log_info!("caw: loaded env overrides from {:?}", path);
    return;
    }
}

/// Load NetEase configuration from the persisted store, then apply .env
/// overrides on top.
pub fn load_netease_config(app: &AppHandle) -> NeteaseConfig {
    let mut config = NeteaseConfig::default();

    if let Ok(store) = app.store(crate::paths::config_path()) {
        let get_bool = |key: &str| store.get(key).and_then(|v| v.as_bool());
        let get_str = |key: &str| {
  store
          .get(key)
     .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        };

        config.enabled = get_bool("netease_enabled").unwrap_or(false);
        config.anonymous_mode = get_bool("netease_anonymous_mode").unwrap_or(true);
 config.app_id = get_str("netease_app_id").unwrap_or_default();
        config.private_key = get_str("netease_private_key").unwrap_or_default();
        if let Some(host) = get_str("netease_api_host") {
      config.api_host = host;
    }
        if let Some(v) = get_str("netease_device_os") {
       config.device.os = v;
  }
     if let Some(v) = get_str("netease_device_channel") {
   config.device.channel = v;
        }
      if let Some(v) = get_str("netease_device_brand") {
     config.device.brand = v;
        }
        if let Some(v) = get_str("netease_device_model") {
       config.device.model = v;
      }
if let Some(v) = get_str("netease_device_id") {
        config.device.device_id = v;
   }
    }

 apply_env_overrides(&mut config);

 // A stable per-install device id; the platform expects a consistent value.
    if config.device.device_id.trim().is_empty() {
        config.device.device_id = default_device_id();
    }

    config
}

/// .env / environment overrides. These win over the persisted store so a dev
/// can point the app at different credentials without touching Settings.
fn apply_env_overrides(config: &mut NeteaseConfig) {
    if let Some(v) = env_var("CAW_NETEASE_APP_ID") {
        config.app_id = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_PRIVATE_KEY") {
        config.private_key = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_API_HOST") {
        config.api_host = v;
    }
  if let Some(v) = env_var("CAW_NETEASE_BIZ_ID") {
        config.biz_id = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_OS") {
 config.device.os = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_CHANNEL") {
        config.device.channel = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_BRAND") {
        config.device.brand = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_DEVICE_TYPE") {
        config.device.device_type = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_MODEL") {
config.device.model = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_APP_VER") {
 config.device.app_ver = v;
    }
    if let Some(v) = env_var("CAW_NETEASE_DEVICE_ID") {
        config.device.device_id = v;
    }

    // Credentials present but the module never switched on: enabling it here
    // matches the intent of putting them in .env in the first place.
    if !config.app_id.trim().is_empty()
      && !config.private_key.trim().is_empty()
        && env_var("CAW_NETEASE_APP_ID").is_some()
    {
    config.enabled = true;
    }
}

/// Derive a stable device id from the machine + user, hashed so nothing
/// identifying leaves the process.
fn default_device_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let seed = format!(
        "{}|{}|{}",
        hostname(),
     std::env::var("USER").unwrap_or_default(),
        std::env::consts::OS,
    );

    let mut hasher = DefaultHasher::new();
  seed.hash(&mut hasher);
    format!("caw_{:016x}", hasher.finish())
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
.map(|s| s.trim().to_string())
   .unwrap_or_else(|_| "unknown".to_string())
}

/// Persist NetEase configuration to the store.
pub fn save_netease_config(app: &AppHandle, config: &NeteaseConfig) {
    if let Ok(store) = app.store(crate::paths::config_path()) {
        use serde_json::Value;
        store.set("netease_enabled", Value::Bool(config.enabled));
        store.set("netease_app_id", Value::String(config.app_id.clone()));
        store.set(
    "netease_private_key",
    Value::String(config.private_key.clone()),
        );
   store.set(
    "netease_anonymous_mode",
       Value::Bool(config.anonymous_mode),
        );
        store.set("netease_api_host", Value::String(config.api_host.clone()));
 store.set("netease_device_os", Value::String(config.device.os.clone()));
        store.set(
          "netease_device_channel",
     Value::String(config.device.channel.clone()),
    );
        store.set(
       "netease_device_brand",
            Value::String(config.device.brand.clone()),
      );
        store.set(
   "netease_device_model",
   Value::String(config.device.model.clone()),
        );
        store.set(
       "netease_device_id",
   Value::String(config.device.device_id.clone()),
        );
    let _ = store.save();
    }
}

/// Persist OAuth tokens so a QR login survives an app restart.
pub fn save_netease_tokens(app: &AppHandle, tokens: &NeteaseTokens) {
    if let Ok(store) = app.store(crate::paths::config_path()) {
        use serde_json::Value;
     store.set(
          "netease_access_token",
    Value::String(tokens.access_token.clone()),
        );
        store.set(
     "netease_refresh_token",
   Value::String(tokens.refresh_token.clone()),
    );
        store.set(
            "netease_token_expires_at",
     Value::Number(tokens.expires_at_ms.into()),
        );
   let _ = store.save();
    }
}

pub fn load_netease_tokens(app: &AppHandle) -> NeteaseTokens {
 if let Ok(store) = app.store(crate::paths::config_path()) {
        NeteaseTokens {
   access_token: store
    .get("netease_access_token")
       .and_then(|v| v.as_str().map(|s| s.to_string()))
           .unwrap_or_default(),
    refresh_token: store
        .get("netease_refresh_token")
       .and_then(|v| v.as_str().map(|s| s.to_string()))
     .unwrap_or_default(),
    expires_at_ms: store
    .get("netease_token_expires_at")
            .and_then(|v| v.as_i64())
   .unwrap_or(0),
}
    } else {
        NeteaseTokens::default()
    }
}
