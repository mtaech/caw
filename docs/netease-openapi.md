# 网易云音乐 OpenAPI 对接说明

> 本文是官方文档的提炼，供 `src-tauri/src/netease/` 实现参考。
> 官方原文：<https://music.163.com/st/sublicense-doc/open-doc>

## 一、两套接口不是一回事

平台上并存两条产品线，**路径前缀和能力完全不同**，选错就一直 404。

| | 转授权（云村交易所） | 个人/用户授权 |
|---|---|---|
| 域名 | `openapi.music.163.com` | `openncm.music.163.com` |
| 路径前缀 | `/openapi/music/authority/*` | `/openapi/music/basic/*` |
| 必需凭据 | appId + 私钥 + **bizId** | appId + 私钥 + **用户 accessToken** |
| 面向 | 签约企业，曲库为合同授权范围 | 个人开发者，操作登录用户自己的账号 |
| 有「每日推荐」吗 | **没有** | 有 |

关键结论：

- **推荐、心动模式、我的歌单、红心、最近播放 —— 只有个人/用户授权那条线有。**
- 转授权线只能拿到「合同授权曲库」内的歌曲列表/详情/播放地址，没有任何个性化能力，且 `bizId` 由商务分配。
- Caw 要做推荐，走 `openncm.music.163.com` + `/openapi/music/basic/*`。

原有代码写的是 `interface.music.163.com/openapi/music/basic/search/song` —— 域名和路径都不对，实测返回
`{"code":404,"message":"接口未找到！"}`。

## 二、签名（两条线通用）

官方 Java 示例的算法，逐字翻译过来就是：

1. 收集公共参数，放进**按 key 升序排序**的 map；
2. 拼成 `k1=v1&k2=v2&...`（**原始值，不做 URL 编码**；`sign` 自身必须排除）；
3. 用 PKCS#8 2048-bit RSA 私钥做 `SHA256WithRSA` 签名；
4. Base64 编码，作为 `sign` 参数一起发出。

```text
待签串 = appId=xxx&bizContent={"limit":30}&device={...}&signType=RSA_SHA256&timestamp=1695697190151
sign   = Base64(RSA_SHA256_Sign(待签串, 私钥))
```

### 公共参数

| 参数 | 必选 | 说明 |
|---|---|---|
| `appId` | 是 | 控制台分配 |
| `timestamp` | 是 | 毫秒时间戳 |
| `signType` | 是 | 固定 `RSA_SHA256` |
| `bizContent` | 是 | 业务参数 JSON 化后的**字符串** |
| `sign` | 是 | 见上 |
| `device` | 视接口 | 设备信息 JSON 化，见下 |
| `accessToken` | 个人线业务接口 | 用户授权令牌 |

官方给的 `device` 格式（转授权线示例）：

```json
{"appVer":"...","brand":"sublicense","channel":"sublicense","clientIp":"...",
 "deviceId":"...","deviceType":"...","model":"...","os":"openapi","osVer":"..."}
```

**注意两个坑：**

1. `device` 里 `clientIp` 缺失会被拒：`code 2511 "缺少clientIp参数"`。需要先拿到公网 IP。
2. `os` / `channel` / `brand` 会被服务端校验，跟 appId 的注册信息不匹配就报
   `code 400 "公共参数校验失败，请检查os/channel/brand等是否符合约定"`。
   **这几个值必须问平台对接人要**，不能自己编。已在 `.env` 留出
   `CAW_NETEASE_OS` / `CAW_NETEASE_CHANNEL` / `CAW_NETEASE_BRAND` 等项。

### 返回格式（统一）

```json
{ "code": 200, "message": "", "data": { } }
```

`code`：200 成功 / 400 业务异常 / 500 服务异常。另有业务码：

- `301 用户未授权当前接口` —— token 没有该接口的 scope（匿名 token 调业务接口必然报这个）
- `1405 非法appId参数`
- `2511 缺少clientIp参数`

## 三、个人/用户授权线（Caw 用这条）

域名 `openncm.music.163.com`。业务接口都要带 `accessToken`。

### 登录换 token

| 用途 | 方法 | 路径 |
|---|---|---|
| 匿名登录 | POST | `/openapi/music/basic/oauth2/login/anonymous` |
| 取二维码 | GET | `/openapi/music/basic/user/oauth2/qrcodekey/get/v2` |
| 轮询扫码结果 | GET | `/openapi/music/basic/oauth2/device/login/qrcode/get` |

匿名登录 `bizContent` 为 `{"clientId": "<appId>"}`，返回：

```json
{"code":200,"data":{"accessToken":"...","refreshToken":"...","expireTime":86400,"scopes":null}}
```

**`scopes` 为 null 的匿名 token 只能用于换取二维码，调任何业务接口都返回 301。**
要用推荐/搜索/歌单，必须走扫码登录拿到用户授权的 token。

### 业务接口

推荐：

| 功能 | 方法 | 路径 | 主要 bizContent |
|---|---|---|---|
| 每日推荐歌曲 | GET | `/openapi/music/basic/recommend/songlist/get/v2` | `limit`, `qualityFlag` |
| 心动模式 | POST | `/openapi/music/basic/song/play/intelligence/get` | `playlistId`*, `songId`*, `count` |
| 场景私人漫游 | POST | `/openapi/music/basic/private/fm/roaming/song/list` | `type`*, `code`*, `limit`* |
| 雷达歌单 | GET | `/openapi/music/basic/playlist/radar/get` | — |

搜索：

| 功能 | 方法 | 路径 | 主要 bizContent |
|---|---|---|---|
| 综合搜索 | GET | `/openapi/music/basic/complex/search` | `keyword`* |
| 搜索歌曲 | GET | `/openapi/music/basic/search/song/get/v3` | `keyword`*, `limit`, `offset` |
| 搜索专辑 | GET | `/openapi/music/basic/search/album/get/v2` | `keyword`*, `limit`, `offset` |
| 搜索歌单 | GET | `/openapi/music/basic/search/playlist/get/v2` | `keyword`*, `limit`, `offset` |

歌单与歌曲：

| 功能 | 方法 | 路径 | 主要 bizContent |
|---|---|---|---|
| 我创建的歌单 | GET | `/openapi/music/basic/playlist/created/get/v2` | `limit`, `offset` |
| 我收藏的歌单 | GET | `/openapi/music/basic/playlist/subed/get/v2` | `limit`, `offset` |
| 我的红心歌单 | POST | `/openapi/music/basic/playlist/star/get/v2` | — |
| 歌单详情 | GET | `/openapi/music/basic/playlist/detail/get/v2` | `playlistId`* |
| 歌单歌曲列表 | GET | `/openapi/music/basic/playlist/song/list/get/v3` | `playlistId`*, `limit`, `offset` |
| 歌曲详情 | GET | `/openapi/music/basic/song/detail/get/v2` | `songId` |
| 歌词 | GET | `/openapi/music/basic/song/lyric/get/v2` | `songId`* |
| 红心/取消红心 | POST | `/openapi/music/basic/playlist/song/like/v2` | `songId`* + `isLike` |
| 最近播放 | GET | `/openapi/music/basic/song/play/record/list` | `limit` |
| 用户信息 | GET | `/openapi/music/basic/user/profile/get/v2` | — |
| 播放上报 | POST | `/openapi/music/basic/play/data/record` | 见官方上报枚举 |

### 歌曲 id 有两种

响应里同时给 `originalId`（明文数字 id）和 `id`（32 位 hex 加密 id）。
**业务接口的 `songId` / `playlistId` 一律传加密 id。**

### 音频等级 `level`

`standard` < `higher` < `exhigh` < `lossless` < `hires`。
配合 `brDownwardCompatible` 决定拿不到目标码率时是否降级。

## 四、转授权线（Caw 暂不使用，备查）

域名 `openapi.music.163.com`，**全部 POST**，`bizContent` 必带 `bizId`。

| 功能 | 路径 |
|---|---|
| 歌曲列表 | `/openapi/music/authority/song/list` |
| 歌曲详情 | `/openapi/music/authority/song/detail` |
| 歌曲播放地址 | `/openapi/music/authority/song/url` |
| 专辑详情 | `/openapi/music/authority/album/detail` |
| 艺人详情 | `/openapi/music/authority/artist/detail` |
| 歌单列表 / 播放地址 / 歌曲列表 | `/openapi/music/authority/playlist/{list,url,song/list}` |
| 标签列表 | `/openapi/music/authority/tag/list` |
| 歌曲版权 | `/openapi/music/authority/song/copyright` |
| 伴奏（K 歌场景） | `/openapi/music/authority/accompany/{detail,details,url,ext/music/url}` |
| 原唱播放地址 | `/openapi/music/authority/origin/url` |
| 裁剪片段 | `/openapi/music/authority/clips/url` |
| 调用量统计 | `/openapi/music/authority/summary/invokerecord/{detail,count}` |
| 播放上报 | `/openapi/music/basic/common/data/record` |

`song/url` 返回是 `Map<歌曲id, {url, br, size, md5, type, code, level, time}>`，
单曲级 `code` 为 404 表示该曲在合同范围内无可用音频。

未配置自有域名时，播放链接默认以 `music-open-data-online.music.126.net` 为前缀。

## 五、密钥要求

- RSA，**2048 bit，PKCS#8**；
- 公钥去掉 `-----BEGIN/END PUBLIC KEY-----` 与换行，压成一行上传控制台；
- 私钥同样压成一行，填入 `.env` 的 `CAW_NETEASE_PRIVATE_KEY`。

```bash
openssl genrsa -out app_private_key.pem 2048
openssl pkcs8 -topk8 -inform PEM -in app_private_key.pem -outform PEM -nocrypt -out app_private_key_pkcs8.pem
openssl rsa -in app_private_key.pem -pubout -out app_public_key.pem
```
