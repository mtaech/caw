# Caw — 项目说明 (Project Instructions)

本地音乐播放器，**纯 Rust 桌面应用**：
GUI 用 `gpui-kit`（GPUI + gpui-component），音频解码用 Symphonia，输出用 cpal。
架构与分阶段计划见 `PLAN.md`（中文，权威文档），gpui-kit 0.7 的 API 速查见
`docs/gpui-kit-api-notes.md`。

## 项目本质 (What this is)

- **语言**: Rust，edition 2024，toolchain 1.99（`rustc --version` 为准）。
- **GUI**: `gpui-kit = "0.7.0"`（crates.io，单一依赖；内含 GPUI / gpui-base / gpui-component / assets）。
  不要直接依赖 `gpui`、`gpui-component`：全部从 `gpui_kit` 命名空间导入。
- **音频**: `symphonia` 解码（FLAC/MP3/WAV/Ogg/AAC/ALAC）→ `cpal` 输出（Linux ALSA/PipeWire）。
- **存储**: SQLite（`rusqlite`，歌单/音乐目录）+ `~/.config/caw/config.json`（偏好与窗口状态）。
- **平台**: Linux 优先；MPRIS 与系统托盘（StatusNotifierItem + com.canonical.dbusmenu）均直接用 zbus 实现（ksni 0.3.7 与 zbus 5.19 不兼容，未使用）。
- **目标平台**: Linux（Fedora 开发）；CI 额外构建 Windows。无 Node/npm/Tauri 代码。

## 常用命令 (Build / run / check)

```bash
cargo run                 # 编译并运行（首次因 gpui-kit 依赖较慢，之后增量）
cargo build               # 仅编译
cargo check --all-targets # 快速类型检查（改完代码先跑这个）
cargo clippy --all-targets# 风格/lint（提交前跑）
cargo fmt                 # 格式化
```

- 当前**没有测试套件**；不要为了让“测试通过”而编造无意义测试。GUI 用 `cargo run` 冒烟。
- 运行产物与日志：`~/.config/caw/`（`config.json`、`caw.db`、`caw.log`、`covers/`）。
- 提交前必须：`cargo fmt`、`cargo clippy --all-targets`（0 error）、`cargo check --all-targets`。

## 架构与约定 (Architecture & conventions)

模块布局（见 `src/`）：

- `main.rs` — 入口：`gpui_kit::application().with_assets(CawAssets)`、`gpui_kit::init`、
  读取 config 应用主题/字体、`open_window`、`Root::new(PlayerApp)`、250ms 轮询定时器。
- `root_view.rs` — `impl Render for PlayerApp`：整体布局（TitleBar + Sidebar + Content +
  PlayerBar）、全局快捷键、模态遮罩（新建歌单 / 添加到歌单）。
- `app.rs` — `PlayerApp`：唯一顶层状态（曲库、过滤索引、播放状态/队列、音量、歌单、
  音乐目录、设置、config），所有 action 方法。
- `config.rs` — `AppConfig`（JSON 持久化：主题、字体、音量、播放模式、关闭行为、
  上次播放/队列快照等），原子写（tmp+rename）。
- `logging.rs` — 文件日志 + 控制台（`log_info!` 等宏，保留 `caw: ` 前缀，2MiB×5 轮转）。
- `db.rs` — SQLite：playlists / playlist_tracks / music_dirs / settings。
- `audio/` — `library.rs`（walkdir 扫描去重）、`decoder.rs`（Symphonia 封装）、
  `player.rs`（cpal 输出线程 + `PlayerCommand`）。
- `models/` — `track.rs`（`Track`/`AudioFormat`）、`playlist.rs`（`PlaybackState`/`PlaybackMode`）。
- `ui/` — `sidebar.rs`、`player_bar.rs`、`icons.rs`（`CawIcon`）、`dialogs.rs`、`marquee.rs`、
  `cover.rs`、`content/{mod,tracks,artists,albums,playlists,folders,settings,queue}.rs`。
- `platform/` — Linux：MPRIS、系统托盘（SNI + DBusMenu）、窗口状态；非 Linux 用 cfg 关闭。
- `paths.rs` / `assets.rs` — `~/.config/caw/` 路径、内嵌 SVG 资源（`CawAssets`）。

### 必须遵守的约定

- **单一根视图**：`PlayerApp` 就是窗口根视图（`impl Render for PlayerApp` 在 `root_view.rs`），
  不要恢复 `CawWindow` 包装。UI 渲染函数签名统一为
  `fn render_x(app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>) -> impl IntoElement`。
- **状态即 Entity**：所有可变状态在 `PlayerApp`；输入框/滑杆用 `Entity<InputState>`/`Entity<SliderState>`。
  状态变化后 `cx.notify()`。
- **gpui-kit 导入**：`use gpui_kit::*;` 只是 GPUI；布局与样式扩展在 `gpui_kit::base`
  （`h_flex`/`v_flex`/`StyledExt`/`FluentBuilder` 在 `gpui_kit::prelude`）。
  `.overflow_y_scroll()` 需要先 `.id(...)`（`StatefulInteractiveElement` 只实现在 `Stateful`）。
- **不要臆造 API**：以 vendored 源码为准（`~/.cargo/registry/src/.../gpui-{kit,base,component}-0.7.0`），
  或 `https://gpui-kit.com/component/{name}.md`。改动前先 `cargo check`。
- **主题令牌**：颜色一律走 `cx.theme()`（无 `card` 令牌，用 `popover`/`secondary`）；
  不写裸 hex。全局字体改 `Theme.font_family`，不要每个 div 设字体。
  `ThemeMode` 只有 Light / Dark（“跟随系统”要在 app 层解析）。
- **音频线程隔离**：解码与 cpal 输出在独立线程，UI 线程绝不能阻塞；跨线程用
  `crossbeam_channel` + `Atomic*`；**cpal 回调里禁止阻塞或重锁**。
- **进度更新**：音频线程推进位置，UI 用 `cx.background_executor().timer(250ms)` 轮询并 `cx.notify()`；
  不要在音频回调里动 Entity。停止/切歌用 generation 计数防止旧解码覆盖新播放。
- **UI 不复制 `Vec<Track>`**：列表用 `Arc<Track>` + 索引视图；避免每行/每次 render 全量克隆。
- **封面加载**：`img(PathBuf)` 走文件系统；`img(String)` 会走 AssetSource 从而加载不到
  `~/.config/caw/covers/*.jpg`。
- **错误处理**：单文件解码失败跳过并记录；播放出错自动下一首；不要让坏文件崩掉扫描。
- **日志**：用 `log_info!/log_warn!/log_error!`（`logging.rs`，控制台保留 `caw: ` 前缀）。
- **键盘**：全局快捷键挂在根视图；输入框聚焦或有模态打开时不得抢占 Space/方向键。
- **平台代码**：MPRIS/托盘/窗口状态不得直接修改 Entity；通过 channel 由 app 定时器消费。

## 文档/注释风格

- 代码注释、commit message、技术文档用**英文**（与现有代码一致）。
- `PLAN.md`、README、AGENTS、CONTEXT 及面向用户的说明用**中文**。
- 回答用户用中文；先给结论/命令，再按需补充解释。
