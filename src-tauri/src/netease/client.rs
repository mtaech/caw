//! NetEase Cloud Music Open Platform API client.
//!
//! Targets the personal / user-authorized product line:
//!   host  openncm.music.163.com
//!   paths /openapi/music/basic/*
//!
//! Every request carries the common params (appId, timestamp, signType,
//! bizContent, device, [accessToken]) plus an RSA-SHA256 signature over them.
//! See docs/netease-openapi.md for the full protocol notes.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use reqwest::Client;
use rsa::RsaPrivateKey;
use serde_json::Value;

use super::sign::{parse_private_key, sign_params, SIGN_TYPE};
use super::types::{
    now_ms, NeteaseConfig, NeteaseConnectionStatus, NeteaseLyricDto, NeteasePlayUrlDto,
    NeteasePlaylistDto, NeteaseQrPollDto, NeteaseQrSessionDto, NeteaseSearchResultDto,
    NeteaseSongDto, NeteaseTokens, NeteaseUserDto,
};

// -- Endpoint paths -------------------------------------------------
const EP_LOGIN_ANONYMOUS: &str = "/openapi/music/basic/oauth2/login/anonymous";
const EP_QRCODE_KEY: &str = "/openapi/music/basic/user/oauth2/qrcodekey/get/v2";
const EP_QRCODE_POLL: &str = "/openapi/music/basic/oauth2/device/login/qrcode/get";
const EP_USER_PROFILE: &str = "/openapi/music/basic/user/profile/get/v2";
const EP_RECOMMEND_DAILY: &str = "/openapi/music/basic/recommend/songlist/get/v2";
const EP_HEARTBEAT: &str = "/openapi/music/basic/song/play/intelligence/get";
const EP_SEARCH_SONG: &str = "/openapi/music/basic/search/song/get/v3";
const EP_PLAYLIST_CREATED: &str = "/openapi/music/basic/playlist/created/get/v2";
const EP_PLAYLIST_SUBED: &str = "/openapi/music/basic/playlist/subed/get/v2";
const EP_PLAYLIST_TRACKS: &str = "/openapi/music/basic/playlist/song/list/get/v3";
const EP_PLAY_RECORD: &str = "/openapi/music/basic/song/play/record/list";
const EP_SONG_LYRIC: &str = "/openapi/music/basic/song/lyric/get/v2";
const EP_SONG_LIKE: &str = "/openapi/music/basic/playlist/song/like/v2";
const EP_SONG_URL: &str = "/openapi/music/basic/song/play/url/get/v2";

#[derive(Clone)]
pub struct NeteaseClient {
 http: Client,
    config: Arc<RwLock<NeteaseConfig>>,
  tokens: Arc<RwLock<NeteaseTokens>>,
    client_ip: Arc<RwLock<Option<String>>>,
    user: Arc<RwLock<Option<NeteaseUserDto>>>,
}

impl NeteaseClient {
    pub fn new(config: NeteaseConfig) -> Self {
        let http = Client::builder()
.timeout(Duration::from_secs(15))
        .user_agent(format!("Caw/{}", env!("CARGO_PKG_VERSION")))
        .build()
     .unwrap_or_else(|_| Client::new());

        Self {
  http,
         config: Arc::new(RwLock::new(config)),
     tokens: Arc::new(RwLock::new(NeteaseTokens::default())),
 client_ip: Arc::new(RwLock::new(None)),
     user: Arc::new(RwLock::new(None)),
   }
    }

    pub fn update_config(&self, new_config: NeteaseConfig) {
        if let Ok(mut cfg) = self.config.write() {
 *cfg = new_config;
        }
    }

    pub fn get_config(&self) -> NeteaseConfig {
        self.config.read().unwrap().clone()
    }

 pub fn set_tokens(&self, tokens: NeteaseTokens) {
        if let Ok(mut t) = self.tokens.write() {
            *t = tokens;
        }
    }

    pub fn get_tokens(&self) -> NeteaseTokens {
        self.tokens.read().unwrap().clone()
    }

    pub fn cached_user(&self) -> Option<NeteaseUserDto> {
        self.user.read().unwrap().clone()
    }

    pub fn is_logged_in(&self) -> bool {
        self.get_tokens().is_valid()
  }

    pub fn logout(&self) {
        if let Ok(mut t) = self.tokens.write() {
        *t = NeteaseTokens::default();
    }
        if let Ok(mut u) = self.user.write() {
  *u = None;
        }
    }


    // -- Preflight --------------------------------------------------

    /// Validate config before spending a network round-trip on a request that
    /// the platform would reject anyway.
    fn preflight(&self, cfg: &NeteaseConfig) -> Result<RsaPrivateKey, String> {
        if !cfg.enabled {
            return Err("网易云音乐模块未开启".to_string());
        }
        if cfg.app_id.trim().is_empty() {
  return Err("未配置 App ID，请在设置中填写".to_string());
        }
        if !cfg.device.is_complete() {
        return Err(
         "device 参数不完整：os / channel / brand 为空。这几个值由开放平台按 appId 注册，\
      需向平台对接人索取后填入 .env（CAW_NETEASE_OS / _CHANNEL / _BRAND）"
    .to_string(),
            );
   }
        parse_private_key(&cfg.private_key)
    }

/// The platform rejects requests whose device param lacks clientIp
    /// (code 2511), so resolve the public IP once and cache it.
    async fn resolve_client_ip(&self) -> String {
   if let Some(ip) = self.client_ip.read().unwrap().clone() {
     return ip;
        }

    let mut resolved = "127.0.0.1".to_string();
    for url in ["https://api.ipify.org?format=json", "https://api.ip.sb/jsonip"] {
  if let Ok(res) = self.http.get(url).send().await {
         if let Ok(json) = res.json::<Value>().await {
    if let Some(ip) = json.get("ip").and_then(|v| v.as_str()) {
    resolved = ip.to_string();
           break;
        }
      }
    }
        }

        if let Ok(mut slot) = self.client_ip.write() {
   *slot = Some(resolved.clone());
        }
        resolved
    }

    // -- Core request -----------------------------------------------

    /// Issue a signed OpenAPI call.
    ///
  /// with_token attaches the user access token; business endpoints all
    /// require it and return code 301 without a properly scoped one (an
    /// anonymous token has scopes: null and will not do).
    async fn request(
 &self,
  path: &str,
        biz: Value,
        method: reqwest::Method,
     with_token: bool,
    ) -> Result<Value, String> {
        let cfg = self.get_config();
        let key = self.preflight(&cfg)?;
     let client_ip = self.resolve_client_ip().await;

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("appId".to_string(), cfg.app_id.clone());
        params.insert("signType".to_string(), SIGN_TYPE.to_string());
        params.insert("timestamp".to_string(), now_ms().to_string());
        params.insert("device".to_string(), cfg.device.to_json(&client_ip));
 params.insert("bizContent".to_string(), biz.to_string());

  if with_token {
            let token = self.get_tokens().access_token;
            if token.trim().is_empty() {
      return Err("尚未登录网易云账号，请先扫码登录".to_string());
   }
            params.insert("accessToken".to_string(), token);
        }

        params.insert("sign".to_string(), sign_params(&params, &key));

     let url = format!("{}{}", cfg.base_url(), path);
        let res = self
         .http
            .request(method, &url)
     .query(&params)
        .header("Referer", "https://music.163.com/")
            .send()
   .await
   .map_err(|e| format!("网络请求失败: {e}"))?;

        let status = res.status();
        let json: Value = res
          .json()
          .await
            .map_err(|e| format!("解析响应失败 [{status}]: {e}"))?;

        let code = json.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
   if code != 200 {
          let msg = json
                .get("message")
        .or_else(|| json.get("msg"))
    .and_then(|m| m.as_str())
                .unwrap_or("未知错误");
  return Err(explain_error(code, msg));
     }

        Ok(json.get("data").cloned().unwrap_or(Value::Null))
  }


    // -- Auth -------------------------------------------------------

  /// Exchange appId credentials for an anonymous token. This token can only
    /// bootstrap the QR login; business endpoints reject it with code 301.
    pub async fn login_anonymous(&self) -> Result<NeteaseTokens, String> {
        let cfg = self.get_config();
        let biz = serde_json::json!({ "clientId": cfg.app_id });
        let data = self
            .request(EP_LOGIN_ANONYMOUS, biz, reqwest::Method::POST, false)
.await?;

        let tokens = parse_tokens(&data);
        if tokens.access_token.is_empty() {
            return Err("匿名登录未返回 accessToken".to_string());
        }
        self.set_tokens(tokens.clone());
        Ok(tokens)
    }

    /// Start a QR login: returns the URL to render as a QR code plus the
    /// uniKey used to poll for the result.
    pub async fn start_qr_login(&self) -> Result<NeteaseQrSessionDto, String> {
    // The qrcode endpoint itself needs a token; an anonymous one suffices.
        if !self.get_tokens().is_valid() {
  self.login_anonymous().await?;
        }

      let biz = serde_json::json!({ "type": 2, "expiredKey": "300" });
        let data = self
            .request(EP_QRCODE_KEY, biz, reqwest::Method::GET, true)
            .await?;

        let uni_key = data
       .get("uniKey")
   .and_then(|v| v.as_str())
          .unwrap_or_default()
          .to_string();
        let qr_code_url = data
  .get("qrCodeUrl")
   .and_then(|v| v.as_str())
      .unwrap_or_default()
            .to_string();

    if uni_key.is_empty() || qr_code_url.is_empty() {
            return Err("获取登录二维码失败".to_string());
        }

   Ok(NeteaseQrSessionDto { uni_key, qr_code_url })
  }

    /// Poll a QR login session once. On success the user token replaces the
    /// anonymous one and the profile is cached.
    pub async fn poll_qr_login(&self, uni_key: &str) -> Result<NeteaseQrPollDto, String> {
        let biz = serde_json::json!({ "uniKey": uni_key });
     let data = match self
   .request(EP_QRCODE_POLL, biz, reqwest::Method::GET, true)
         .await
        {
            Ok(d) => d,
      Err(e) => {
    return Ok(NeteaseQrPollDto {
     status: "error".to_string(),
  message: e,
         user: None,
     })
          }
      };

        let tokens = parse_tokens(&data);
        if !tokens.access_token.is_empty() {
      self.set_tokens(tokens);
        let user = self.fetch_user_profile().await.ok();
    if let Ok(mut slot) = self.user.write() {
        *slot = user.clone();
            }
            return Ok(NeteaseQrPollDto {
     status: "confirmed".to_string(),
         message: "登录成功".to_string(),
        user,
        });
        }

     // No token yet - surface the intermediate scan state when present.
let code = data
      .get("code")
        .and_then(|c| c.as_i64())
    .or_else(|| data.get("qrCodeStatus").and_then(|c| c.as_i64()))
    .unwrap_or(0);
  let (status, message) = match code {
        802 => ("scanned", "已扫码，请在手机上确认"),
 800 => ("expired", "二维码已过期，请刷新"),
       _ => ("waiting", "等待扫码"),
     };

  Ok(NeteaseQrPollDto {
            status: status.to_string(),
            message: message.to_string(),
            user: None,
        })
    }

    pub async fn fetch_user_profile(&self) -> Result<NeteaseUserDto, String> {
        let data = self
            .request(EP_USER_PROFILE, serde_json::json!({}), reqwest::Method::GET, true)
            .await?;

        Ok(NeteaseUserDto {
        id: str_field(&data, "id"),
            original_id: data.get("originalId").and_then(|v| v.as_i64()),
       nickname: data
      .get("nickname")
        .and_then(|v| v.as_str())
       .unwrap_or("网易云用户")
   .to_string(),
     avatar_url: data
                .get("avatarUrl")
       .and_then(|v| v.as_str())
      .map(|s| s.to_string()),
   vip: data
   .get("vipDetail")
 .and_then(|v| v.as_array())
   .map(|a| !a.is_empty())
       .unwrap_or(false),
      })
    }

    /// Verify credentials end to end.
    pub async fn test_connection(&self) -> NeteaseConnectionStatus {
        let cfg = self.get_config();
        if let Err(e) = self.preflight(&cfg) {
       return NeteaseConnectionStatus {
                ok: false,
                message: e,
      user_name: None,
     };
        }

    if self.is_logged_in() {
            return match self.fetch_user_profile().await {
              Ok(user) => NeteaseConnectionStatus {
         ok: true,
            message: "已登录，连接正常".to_string(),
         user_name: Some(user.nickname),
     },
  Err(e) => NeteaseConnectionStatus {
        ok: false,
         message: e,
  user_name: None,
        },
            };
 }

      match self.login_anonymous().await {
      Ok(_) => NeteaseConnectionStatus {
      ok: true,
          message: "凭证有效（匿名鉴权通过）。推荐与歌单需扫码登录后可用".to_string(),
     user_name: None,
          },
            Err(e) => NeteaseConnectionStatus {
     ok: false,
      message: e,
              user_name: None,
      },
        }
 }


    // -- Content ----------------------------------------------------

 /// Daily recommended songs - the centerpiece of the 推荐 tab.
    pub async fn recommend_daily(&self, limit: u32) -> Result<Vec<NeteaseSongDto>, String> {
        let biz = serde_json::json!({ "limit": limit, "qualityFlag": true });
        let data = self
            .request(EP_RECOMMEND_DAILY, biz, reqwest::Method::GET, true)
 .await?;
    Ok(parse_song_list(&data))
    }

    /// Heartbeat mode: songs recommended from a seed track in a playlist.
    pub async fn heartbeat(
    &self,
        playlist_id: &str,
        song_id: &str,
        count: u32,
    ) -> Result<Vec<NeteaseSongDto>, String> {
   let biz = serde_json::json!({
         "playlistId": playlist_id,
      "songId": song_id,
            "count": count,
  });
        let data = self
    .request(EP_HEARTBEAT, biz, reqwest::Method::POST, true)
            .await?;
        Ok(parse_song_list(&data))
    }

    /// Recently played songs.
    pub async fn play_history(&self, limit: u32) -> Result<Vec<NeteaseSongDto>, String> {
        let biz = serde_json::json!({ "limit": limit, "qualityFlag": true });
        let data = self
            .request(EP_PLAY_RECORD, biz, reqwest::Method::GET, true)
       .await?;
        Ok(parse_song_list(&data))
    }

    pub async fn search_songs(
     &self,
        query: &str,
        page: u32,
        limit: u32,
    ) -> Result<NeteaseSearchResultDto, String> {
        let q = query.trim();
if q.is_empty() {
       return Ok(NeteaseSearchResultDto {
      songs: Vec::new(),
           total: 0,
            });
        }

        let offset = page.saturating_sub(1) * limit;
        let biz = serde_json::json!({
            "keyword": q,
            "limit": limit,
      "offset": offset,
            "qualityFlag": true,
        });
      let data = self
            .request(EP_SEARCH_SONG, biz, reqwest::Method::GET, true)
            .await?;

        let songs = parse_song_list(&data);
        let total = data
            .get("total")
 .and_then(|v| v.as_u64())
   .unwrap_or(songs.len() as u64) as u32;

  Ok(NeteaseSearchResultDto { songs, total })
    }

    /// Playlists the user created plus the ones they subscribed to.
    pub async fn my_playlists(&self, limit: u32) -> Result<Vec<NeteasePlaylistDto>, String> {
        let biz = serde_json::json!({ "limit": limit, "offset": 0 });

        let mut out = Vec::new();
        let created = self
  .request(EP_PLAYLIST_CREATED, biz.clone(), reqwest::Method::GET, true)
        .await?;
        out.extend(parse_playlist_list(&created));

  // A failure here should not discard the playlists we already have.
        match self
            .request(EP_PLAYLIST_SUBED, biz, reqwest::Method::GET, true)
            .await
        {
            Ok(subed) => out.extend(parse_playlist_list(&subed)),
 Err(e) => crate::log_info!("caw: netease subscribed playlists failed: {e}"),
        }

        Ok(out)
    }

    pub async fn playlist_tracks(
      &self,
        playlist_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<NeteaseSongDto>, String> {
        let biz = serde_json::json!({
            "playlistId": playlist_id,
            "limit": limit,
            "offset": offset,
            "qualityFlag": true,
        });
        let data = self
            .request(EP_PLAYLIST_TRACKS, biz, reqwest::Method::GET, true)
 .await?;
        Ok(parse_song_list(&data))
    }

    pub async fn set_like(&self, song_id: &str, like: bool) -> Result<(), String> {
        let biz = serde_json::json!({ "songId": song_id, "isLike": like });
        self.request(EP_SONG_LIKE, biz, reqwest::Method::POST, true)
  .await?;
        Ok(())
    }

    pub async fn get_lyric(&self, song_id: &str) -> Result<NeteaseLyricDto, String> {
  let biz = serde_json::json!({ "songId": song_id });
        let data = self
       .request(EP_SONG_LYRIC, biz, reqwest::Method::GET, true)
            .await?;

    let lyric = data
          .pointer("/lrc/lyric")
          .or_else(|| data.pointer("/lyric"))
            .and_then(|v| v.as_str())
  .unwrap_or("")
     .to_string();
        let tlyric = data
    .pointer("/tlyric/lyric")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

 Ok(NeteaseLyricDto { lyric, tlyric })
    }

    /// Download a remote audio URL into a temp file so the local file-based
    /// player can decode it. Streams are CDN-backed and expire, so we fetch
    /// to disk rather than trying to push bytes into the decode pipeline.
 pub async fn download_to_file(&self, url: &str, dest: &std::path::Path) -> Result<(), String> {
  let res = self
        .http
      .get(url)
         .send()
       .await
     .map_err(|e| format!("下载音频流失败: {e}"))?;

        if !res.status().is_success() {
   return Err(format!("下载音频流失败，状态码 [{}]", res.status()));
        }

  let bytes = res.bytes().await.map_err(|e| format!("读取音频流失败: {e}"))?;
 std::fs::write(dest, &bytes).map_err(|e| format!("写入临时音频文件失败: {e}"))?;
 Ok(())
    }

    /// Resolve a streamable URL for a song.
    pub async fn get_song_url(&self, song_id: &str) -> Result<Option<NeteasePlayUrlDto>, String> {
  let biz = serde_json::json!({
            "songId": song_id,
   "level": "exhigh",
            "brDownwardCompatible": true,
        });
    let data = self
  .request(EP_SONG_URL, biz, reqwest::Method::GET, true)
         .await?;

        // Response may be an object, a single-element array, or a map keyed by
        // song id depending on endpoint version.
     let node = if data.is_array() {
         data.get(0).cloned().unwrap_or(Value::Null)
        } else if data.get("url").is_some() {
            data.clone()
   } else {
            data.get(song_id).cloned().unwrap_or(data.clone())
     };

        let url = node.get("url").and_then(|u| u.as_str()).unwrap_or_default();
        if url.is_empty() {
         return Ok(None);
}

        Ok(Some(NeteasePlayUrlDto {
            url: url.to_string(),
     format: node
           .get("type")
          .and_then(|f| f.as_str())
         .unwrap_or("mp3")
           .to_string(),
            bitrate: node.get("br").and_then(|b| b.as_u64()).unwrap_or(0) as u32,
    size: node.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
     level: node
.get("level")
     .and_then(|l| l.as_str())
    .map(|s| s.to_string()),
  gain: node.get("gain").and_then(|g| g.as_f64()),
            peak: node.get("peak").and_then(|p| p.as_f64()),
        }))
  }
}


// -- Parsing helpers ------------------------------------------------

/// Turn platform error codes into something a user can act on.
fn explain_error(code: i64, msg: &str) -> String {
    match code {
      301 => format!("{msg}（该接口需要用户授权，请先扫码登录；匿名 token 无权限）"),
        400 if msg.contains("os/channel/brand") => format!(
      "{msg}\n提示：device 的 os/channel/brand 必须与 appId 注册信息一致，\
      请在 .env 配置 CAW_NETEASE_OS / _CHANNEL / _BRAND"
        ),
      1405 => format!("{msg}（appId 无效，请检查配置）"),
        2511 => format!("{msg}（device 缺少 clientIp，通常是取公网 IP 失败）"),
        _ => format!("接口错误 [{code}]: {msg}"),
    }
}

fn parse_tokens(data: &Value) -> NeteaseTokens {
    let access_token = data
        .get("accessToken")
   .and_then(|v| v.as_str())
        .unwrap_or_default()
      .to_string();
 let refresh_token = data
   .get("refreshToken")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    // expireTime is a duration in seconds, not an absolute timestamp.
 let expire_secs = data
        .get("expireTime")
        .and_then(|v| v.as_i64())
        .unwrap_or(86400);

    NeteaseTokens {
      access_token,
        refresh_token,
        expires_at_ms: now_ms() + expire_secs * 1000,
    }
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
   .map(|x| match x {
            Value::String(s) => s.clone(),
  Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

/// Song lists come back under several different shapes depending on endpoint.
fn parse_song_list(data: &Value) -> Vec<NeteaseSongDto> {
    let items = if data.is_array() {
        data.as_array().cloned()
 } else {
        data.get("songs")
    .or_else(|| data.get("list"))
    .or_else(|| data.get("songList"))
   .or_else(|| data.get("records"))
        .and_then(|v| v.as_array())
   .cloned()
    };

    items.unwrap_or_default().iter().map(parse_song).collect()
}

fn parse_song(item: &Value) -> NeteaseSongDto {
    // Some endpoints nest the track under "song" / "simpleSong".
    let s = item
        .get("song")
  .or_else(|| item.get("simpleSong"))
        .unwrap_or(item);

let artist = s
        .get("artists")
        .or_else(|| s.get("fullArtists"))
        .or_else(|| s.get("ar"))
     .and_then(|a| a.as_array())
  .map(|arr| {
            arr.iter()
              .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
    .collect::<Vec<_>>()
         .join(" / ")
        })
        .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "未知艺人".to_string());

    let duration_ms = s
        .get("duration")
        .or_else(|| s.get("dt"))
    .and_then(|d| d.as_u64())
   .unwrap_or(0);

    NeteaseSongDto {
        id: str_field(s, "id"),
  original_id: s.get("originalId").and_then(|v| v.as_i64()),
   title: s
         .get("name")
      .or_else(|| s.get("title"))
            .and_then(|v| v.as_str())
       .unwrap_or("未知歌曲")
            .to_string(),
        artist,
     album: s
            .pointer("/album/name")
       .or_else(|| s.pointer("/al/name"))
            .and_then(|v| v.as_str())
  .unwrap_or("未知专辑")
         .to_string(),
  album_id: s
    .pointer("/album/id")
            .and_then(|v| v.as_str())
            .map(|x| x.to_string()),
        duration_secs: duration_ms / 1000,
      pic_url: s
            .get("coverImgUrl")
            .or_else(|| s.pointer("/album/picUrl"))
            .or_else(|| s.pointer("/al/picUrl"))
   .and_then(|v| v.as_str())
            .map(|x| x.to_string()),
        liked: s.get("liked").and_then(|v| v.as_bool()).unwrap_or(false),
        vip_only: s
            .get("vipFlag")
            .or_else(|| s.get("vipPlayFlag"))
            .and_then(|v| v.as_bool())
     .unwrap_or(false),
        playable: s.get("playFlag").and_then(|v| v.as_bool()).unwrap_or(true),
        gain: s.get("gain").and_then(|v| v.as_f64()),
   peak: s.get("peak").and_then(|v| v.as_f64()),
 }
}

fn parse_playlist_list(data: &Value) -> Vec<NeteasePlaylistDto> {
    let items = if data.is_array() {
        data.as_array().cloned()
    } else {
   data.get("playlists")
      .or_else(|| data.get("list"))
        .or_else(|| data.get("records"))
 .and_then(|v| v.as_array())
     .cloned()
    };

    items
        .unwrap_or_default()
        .iter()
    .map(|p| NeteasePlaylistDto {
      id: str_field(p, "id"),
    original_id: p.get("originalId").and_then(|v| v.as_i64()),
            name: p
              .get("name")
  .or_else(|| p.get("playlistName"))
        .and_then(|v| v.as_str())
       .unwrap_or("未命名歌单")
           .to_string(),
         cover_url: p
    .get("coverImgUrl")
      .or_else(|| p.get("picUrl"))
    .and_then(|v| v.as_str())
             .map(|s| s.to_string()),
            track_count: p
                .get("trackCount")
    .or_else(|| p.get("songCount"))
    .and_then(|v| v.as_u64())
       .unwrap_or(0) as u32,
            description: p
          .get("description")
      .and_then(|v| v.as_str())
     .map(|s| s.to_string()),
  creator: p
      .pointer("/creator/nickname")
         .and_then(|v| v.as_str())
 .map(|s| s.to_string()),
        })
   .collect()
}
