//! Data Transfer Objects and types for the NetEase Cloud Music module.
use serde::{Deserialize, Serialize};

/// Default host for the personal / user-authorized product line
/// (`/openapi/music/basic/*`). See docs/netease-openapi.md.
pub const DEFAULT_API_HOST: &str = "openncm.music.163.com";

/// Device fingerprint sent as the `device` common param.
///
/// The platform validates `os` / `channel` / `brand` against what the appId is
/// registered for; a mismatch is rejected with
/// `code 400 公共参数校验失败，请检查os/channel/brand等是否符合约定`.
/// These therefore cannot be invented — they come from config/.env.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseDevice {
    pub device_type: String,
    pub os: String,
  pub channel: String,
  pub brand: String,
    pub model: String,
    pub app_ver: String,
    pub os_ver: String,
 pub device_id: String,
}

impl Default for NeteaseDevice {
    fn default() -> Self {
        Self {
        device_type: "openapi".to_string(),
     os: String::new(),
     channel: String::new(),
     brand: String::new(),
      model: String::new(),
app_ver: env!("CARGO_PKG_VERSION").to_string(),
     os_ver: std::env::consts::OS.to_string(),
            device_id: String::new(),
        }
    }
}

impl NeteaseDevice {
    /// True when the platform-validated fields are all present.
    pub fn is_complete(&self) -> bool {
        !self.os.trim().is_empty()
    && !self.channel.trim().is_empty()
          && !self.brand.trim().is_empty()
    }

    /// Serialize to the JSON the `device` common param expects. `client_ip` is
  /// required — omitting it is rejected with `code 2511 缺少clientIp参数`.
    pub fn to_json(&self, client_ip: &str) -> String {
        serde_json::json!({
            "deviceType": self.device_type,
          "os": self.os,
       "appVer": self.app_ver,
     "channel": self.channel,
            "model": self.model,
    "brand": self.brand,
            "osVer": self.os_ver,
 "clientIp": client_ip,
"deviceId": self.device_id,
      })
        .to_string()
    }
}

/// NetEase configuration persisted on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseConfig {
    pub enabled: bool,
    pub app_id: String,
    pub private_key: String,
    pub anonymous_mode: bool,
    pub api_host: String,
    pub device: NeteaseDevice,
    /// Only used by the B2B sublicense line (`/openapi/music/authority/*`).
  pub biz_id: String,
}

impl Default for NeteaseConfig {
    fn default() -> Self {
        Self {
         enabled: false,
          app_id: String::new(),
     private_key: String::new(),
      anonymous_mode: true,
       api_host: DEFAULT_API_HOST.to_string(),
            device: NeteaseDevice::default(),
          biz_id: String::new(),
        }
    }
}

impl NeteaseConfig {
 pub fn base_url(&self) -> String {
  let host = if self.api_host.trim().is_empty() {
       DEFAULT_API_HOST
        } else {
            self.api_host.trim()
};
        if host.starts_with("http://") || host.starts_with("https://") {
    host.trim_end_matches('/').to_string()
   } else {
  format!("https://{host}")
    }
    }
}

/// Safe DTO returned to frontend (without exposing the raw private key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseConfigDto {
    pub enabled: bool,
    pub app_id: String,
    pub has_private_key: bool,
  pub anonymous_mode: bool,
    pub api_host: String,
    /// False when os/channel/brand are missing, which makes every call fail.
    pub device_complete: bool,
    /// True when a user access token is present (QR login completed).
    pub logged_in: bool,
    pub user_name: Option<String>,
}

/// Request payload to update NetEase configuration from Settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNeteaseConfigDto {
    pub enabled: bool,
    pub app_id: Option<String>,
    pub private_key: Option<String>,
    pub anonymous_mode: Option<bool>,
}

/// Track/Song returned from NetEase search / recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseSongDto {
  /// Encrypted 32-hex id. This is what every business endpoint expects.
    pub id: String,
    /// Plaintext numeric id, useful for deep links only.
    pub original_id: Option<i64>,
    pub title: String,
pub artist: String,
    pub album: String,
    pub album_id: Option<String>,
    pub duration_secs: u64,
    pub pic_url: Option<String>,
    pub liked: bool,
    pub vip_only: bool,
    pub playable: bool,
    pub gain: Option<f64>,
    pub peak: Option<f64>,
}

/// Search result container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseSearchResultDto {
    pub songs: Vec<NeteaseSongDto>,
    pub total: u32,
}

/// A playlist summary card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteasePlaylistDto {
    pub id: String,
    pub original_id: Option<i64>,
    pub name: String,
    pub cover_url: Option<String>,
    pub track_count: u32,
    pub description: Option<String>,
    pub creator: Option<String>,
}

/// Playback URL DTO for streaming/playing NetEase songs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteasePlayUrlDto {
    pub url: String,
 pub format: String,
    pub bitrate: u32,
    pub size: u64,
    pub level: Option<String>,
    pub gain: Option<f64>,
    pub peak: Option<f64>,
}

/// Lyric DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseLyricDto {
  pub lyric: String,
pub tlyric: Option<String>,
}

/// NetEase connection test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseConnectionStatus {
    pub ok: bool,
    pub message: String,
    pub user_name: Option<String>,
}

/// Logged-in user profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseUserDto {
    pub id: String,
    pub original_id: Option<i64>,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub vip: bool,
}

/// QR login session handed to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseQrSessionDto {
    pub uni_key: String,
    pub qr_code_url: String,
}

/// Result of polling a QR login session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeteaseQrPollDto {
    /// One of: waiting | scanned | confirmed | expired | error
  pub status: String,
    pub message: String,
    pub user: Option<NeteaseUserDto>,
}

/// Persisted OAuth tokens.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NeteaseTokens {
    pub access_token: String,
 pub refresh_token: String,
    /// Unix epoch millis when `access_token` stops being valid.
    pub expires_at_ms: i64,
}

impl NeteaseTokens {
    pub fn is_valid(&self) -> bool {
     !self.access_token.trim().is_empty() && self.expires_at_ms > now_ms()
    }
}

pub fn now_ms() -> i64 {
  std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
   .map(|d| d.as_millis() as i64)
  .unwrap_or(0)
}
