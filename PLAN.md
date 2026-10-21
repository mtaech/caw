# PLAN.md — Caw 迁移计划：Tauri v2 + Vue 3 → 纯 Rust gpui-kit

> 本文档是本次迁移的权威计划（source of truth）。实施过程中如与本文冲突，以本文为准；
> 若必须偏离，先更新本文并在团队频道说明。

## 一、目标

把 Caw 从 **Tauri v2 + Vue 3 + TypeScript + Tailwind** 迁移为**纯 Rust 单 crate 桌面应用**：

- GUI：`gpui-kit` 0.7（内含 GPUI / gpui-base / gpui-component / gpui-kit-assets）
- 音频：Symphonia 解码 + cpal 输出（沿用旧后端逻辑）
- 存储：SQLite（rusqlite，曲库歌单）+ `~/.config/caw/config.json`（偏好设置）
- 平台：Linux 优先（MPRIS、系统托盘、日志、窗口状态）
- 移除：`src-tauri/`、`src/*.vue`、`package.json`、`vite.config.ts`、npm/Tauri CI

**验收（硬标准）**

1. `cargo check --all-targets` 0 error；`cargo clippy --all-targets` 无 error（warning 可接受但尽量清）。
2. `cargo fmt --check` 通过。
3. `cargo run` 能启动窗口：侧栏/内容区/播放栏可见，扫描曲库、播放/暂停/上下首/音量/进度可用。
4. 功能对齐旧版（见 §四 功能清单），无 Tauri/npm/node 引用残留。
5. CI（`.github/workflows/build-rpm.yml`）改为 cargo 构建并在本地可复现。
6. 文档（README / CONTEXT / AGENTS / 本文件）与最终实现一致。

## 二、当前状态（迁移快照）

- HEAD `87e8833 chore: remove netease integration before gpui-kit migration`。
- 工作树已完成**删除侧**：99 个 Tauri/Vue 文件处于 staged deletion（`git status` 中 D），**尚未提交**。
- 新增侧为未跟踪的纯 Rust 实现（`src/`，约 4160 行）：
  `main.rs / app.rs / assets.rs / paths.rs / db.rs / audio/* / models/* / ui/*`，
  已写完 6 个视图、播放栏、侧栏、图标、播放器线程的**初稿**。
- `cargo check` 当前 **95 个编译错误**，几乎全部是 gpui-kit 0.7 API 漂移（导入路径、
  Theme token 名、trait 未引入、AsyncApp/Context 混用等），不是设计问题。
- 已有能力：后台扫描、Symphonia 解码、cpal 播放、队列/顺序/随机/循环、搜索过滤、
  全部音乐/艺术家/专辑/歌单/文件夹/设置视图、SQLite 歌单与音乐目录持久化。
- 缺失能力：主题切换（亮/暗/系统）、字体设置、窗口关闭确认/托盘/MPRIS/日志/窗口状态、
  歌单对话框（创建/删除/加歌）、队列抽屉的拖拽排序与历史、表格列宽拖拽、
  长标题跑马灯、无封面渐变占位、原生目录选择器、CI 与文档更新。

## 三、目标架构

```text
src/
├── main.rs              # bootstrap: application().with_assets(CawAssets).run / init / open_window
├── app.rs               # PlayerApp：唯一顶层状态 + 播放/曲库/歌单/设置所有 action
├── root_view.rs         # impl Render for PlayerApp：整体布局、快捷键、全局 overlays
├── assets.rs            # CawAssets（内嵌 svg）+ 合并 gpui-kit 默认资源
├── paths.rs             # ~/.config/caw/ 路径
├── config.rs            # [P2] AppConfig JSON：主题/字体/音量/播放模式/窗口状态/关闭行为
├── logging.rs           # [P3] 文件日志（替换裸 eprintln）
├── db.rs                # SQLite：playlists / playlist_tracks / music_dirs / settings
├── models/{track,playlist}.rs
├── audio/{library,decoder,player}.rs
├── platform/{mod,mpris,tray,window_state}.rs   # [P3]
└── ui/
    ├── mod.rs
    ├── icons.rs         # CawIcon -> IconNamed
    ├── sidebar.rs
    ├── player_bar.rs
    ├── dialogs.rs       # [P2] 统一 confirm / playlist / 关闭确认（window.open_dialog）
    ├── marquee.rs       # [P2] 长文本自动滚动
    ├── cover.rs         # [P2] 封面 + 确定性渐变占位
    └── content/
        ├── mod.rs       # 按 nav/detail 分发
        ├── tracks.rs    # 全部音乐 + 曲目表
        ├── artists.rs
        ├── albums.rs
        ├── playlists.rs
        ├── folders.rs
        ├── settings.rs
        └── queue.rs
```

### 3.1 根视图约定（关键决策）

- **`PlayerApp` 自身实现 `Render`**（`src/root_view.rs`），不再用 `CawWindow` 包一层。
  窗口 `Root::new(app_entity, window, cx)`；位置轮询定时器在 `main.rs` 拿到 entity 后 `cx.spawn`。
- UI 渲染函数签名保持：
  ```rust
  pub fn render_sidebar(app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>) -> impl IntoElement
  pub fn render_content(app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>) -> impl IntoElement
  pub fn render_player_bar(app: &PlayerApp, window: &mut Window, cx: &mut Context<PlayerApp>) -> impl IntoElement
  ```
  根视图以 `let app: &PlayerApp = self;` 只读访问，handler 一律 `cx.entity().clone()` 后 `entity.update(cx, ...)`。
- 快捷键：根节点 `.track_focus` / `.on_key_down`，但**搜索框聚焦时不得抢占 Space/箭头**；
  用 `app.search_input.read(cx).focus_handle(cx).is_focused(window)` 判断。
- 主题：启动时读 config → `Theme::change(mode, None, cx)`；字体写入 `Theme::global_mut(cx).font_family`。
- 模态框优先用 gpui-kit 的 `window.open_dialog(...)` / `open_alert_dialog(...)` /
  `window.push_notification(...)`，不再手写绝对定位遮罩。

### 3.2 给 UI 层的最小 PlayerApp 接口（core 负责实现/维护）

UI 已使用、需保持稳定的成员：
`library, filtered_indices, search_query, search_input, selected_nav, detail_target,
selected_track_id, playback_state, playback_mode, current_track, queue, queue_index,
position, duration, volume, is_muted, seeking, progress_slider, volume_slider, db,
playlists, music_dirs, scanning, show_queue`；
方法：`rescan, apply_filter, select_nav, open_detail, back_to_list, play_track, play_all,
toggle_play_pause, next_track, prev_track, seek_to, set_volume, toggle_mute,
cycle_playback_mode, update_position, toggle_queue, play_queue_index, clear_queue,
create_playlist, delete_playlist, add_track_to_playlist, remove_track_from_playlist,
add_music_dir, remove_music_dir`。

新增（P2）：
```rust
pub fn set_theme_mode(&mut self, mode: ThemeMode, cx: &mut Context<Self>);
pub fn set_font_family(&mut self, family: SharedString, cx: &mut Context<Self>);
pub fn update_config(&mut self, f: impl FnOnce(&mut AppConfig));   // 脏标记 + 节流落盘
pub fn playlist_tracks(&self, id: i64) -> Vec<Arc<Track>>;         // 或 UI 直接 app.db.get_playlist_tracks
```
**任何 UI 需要而 app.rs 没有的接口，先给 core 发消息，不要直接改 app.rs。**

## 四、功能对齐清单（旧版 README/UI-POLISH 提取）

| # | 功能 | 现状 | 负责 |
|---|------|------|------|
| 1 | 扫描多目录 + 去重 + 后台扫描 | 有初稿 | core |
| 2 | Symphonia 解码 / cpal 输出 / 暂停 / seek / 音量 | 有初稿 | core |
| 3 | 顺序 / 随机 / 列表循环 / 单曲循环 | 有初稿 | core |
| 4 | 搜索过滤（标题/歌手/专辑） | 有初稿 | content |
| 5 | 全部音乐表（当前行高亮、双击播放、行内操作） | 初稿 | content |
| 6 | 艺术家列表 + 详情 | 初稿 | content |
| 7 | 专辑网格 + 详情 + 封面 | 初稿 | content |
| 8 | 文件夹视图 + 面包屑 | 初稿 | content |
| 9 | 歌单：创建 / 删除 / 加歌 / 移除 / 详情 | 初稿缺对话框 | content + chrome |
| 10 | 队列抽屉：查看 / 跳播 / 清空 /（P2 拖拽排序、历史、存为歌单） | 初稿 | content |
| 11 | 侧栏导航 + 歌单列表 + 扫描状态 | 初稿 | chrome |
| 12 | 播放栏（封面、歌名/歌手、进度、传输、模式、音量、队列按钮） | 初稿 | chrome |
| 13 | 设置：音乐目录管理、关于 | 初稿 | content |
| 14 | 设置：主题（亮/暗/系统）、字体 | 缺 | content + core |
| 15 | 关闭确认 / 关闭到托盘 | 缺 | chrome + platform |
| 16 | 系统托盘 | 缺 | platform |
| 17 | MPRIS（Linux 媒体键/桌面小部件） | 缺 | platform |
| 18 | 文件日志（轮转） | 缺 | platform |
| 19 | 窗口位置/尺寸/状态持久化 | 缺 | platform |
| 20 | 长标题跑马灯 / 封面骨架 / 渐变占位 | 缺 | chrome + content |
| 21 | 键盘快捷键（空格、←/→、↑/↓，编辑时不抢占） | 有初稿 | Lead |
| 22 | 原生目录选择器（rfd）或路径输入兜底 | 缺 | content |

## 五、阶段与里程碑

### P0 — 计划与基建（Lead，进行中）
- 写 PLAN.md / docs/gpui-kit-api-notes.md；建任务板；拉起团队。
- Cargo.toml 依赖审计；确认 gpui-kit 0.7 实际 API。

### P1 — 编译基线 + 结构拆分（全员并行，目标：cargo check 0 error）
- **Lead**：`main.rs` 重写（root view 直挂 PlayerApp、资源、主题初始化、定时器）、
  `root_view.rs`、`ui/mod.rs`、`icons.rs`、`assets.rs`；集成与收口。
- **content**：修 `src/ui/content.rs` 全部错误 → 绿后拆分为 `src/ui/content/*`，保持绿。
- **chrome**：修 `src/ui/sidebar.rs`、`src/ui/player_bar.rs`。
- **core**：修 `src/app.rs`、`src/audio/**`、`src/models/**`、`src/db.rs`，
  `app.rs` 里 `cx.spawn`/`decode_file` 借用与线程问题。
- **platform**（可与 P1 并行，后接入）：`src/logging.rs` 初版 + 调研 MPRIS/托盘方案，
  写清依赖清单发给 Lead。
- 完成标志：`cargo check --all-targets` 0 error（允许 warning）。

### P2 — 功能补齐与重构（目标：功能对齐 §四）
- content：拆模块后补 5/6/7/8/9/10/13/14/22 缺口；对话框统一走 `dialogs.rs`。
- chrome：`dialogs.rs`（confirm/create playlist/add-to-playlist/close confirm）、
  `marquee.rs`、播放栏细节、侧栏歌单区（SidebarGroup 无 header，用 suffix 或自定义行）。
- core：`config.rs` + 持久化（主题/字体/音量/模式/窗口状态/关闭行为）、歌单接口补全。
- Lead：`root_view.rs` 接入 dlgs/主题/字体/快捷键屏蔽、集成与 `cargo check` 收口。

### P3 — 平台集成（Linux 优先）
- `logging.rs` 文件日志（`caw: ` 前缀保留）+ 轮转。
- MPRIS：zbus（纯 Rust）实现 org.mpris.MediaPlayer2 + Player 接口，映射播放/暂停/上下首/seek/音量。
- 托盘：ksni（StatusNotifierItem，走 zbus，无需 GTK 主循环）；菜单：显示/播放暂停/上下首/退出。
- 关闭行为：关闭到托盘 + 退出确认 dialog；窗口 bounds 持久化。
- 依赖由 Lead 统一加入 Cargo.toml。

### P4 — 质量与交付
- `cargo fmt` / `cargo clippy --all-targets` / `cargo check` 全绿。
- `cargo run` 启动验证（DISPLAY=:1 可用）；修 UI 崩溃/panic。
- 文档：README（技术栈/命令/功能）、CONTEXT.md（术语更新）、AGENTS.md（架构/命令）、
  PLAN.md（状态与后续）。
- CI：`.github/workflows/build-rpm.yml` 改为 cargo build + 产物打包；删除 Tauri/Node 步骤。
- 清理：assets 重复图标、`scripts/` 中 Tauri 脚本、`.gitignore`、git 提交。
- Lead 最终 review diff + 提交。

## 六、团队与写作用域（advisory lock）

| 成员 | 写作用域 | 任务 |
|------|----------|------|
| lead | `src/main.rs`, `src/root_view.rs`, `src/ui/mod.rs`, `src/ui/icons.rs`, `src/assets.rs`, `Cargo.toml`, `PLAN.md`, `docs/**`, `.github/**`, `README.md`, `AGENTS.md`, `CONTEXT.md` | 基建 + 集成 + 收口 + 提交 |
| content | `src/ui/content.rs` → `src/ui/content/**`, `src/ui/cover.rs` | 6 个内容视图 |
| chrome | `src/ui/sidebar.rs`, `src/ui/player_bar.rs`, `src/ui/dialogs.rs`, `src/ui/marquee.rs` | 侧栏/播放栏/对话框 |
| core | `src/app.rs`, `src/audio/**`, `src/models/**`, `src/db.rs`, `src/paths.rs`, `src/config.rs` | 状态/音频/持久化 |
| platform | `src/logging.rs`, `src/platform/**` | 日志/MPRIS/托盘/窗口状态 |

## 七、协作规则（必须遵守）

1. **只改自己写作用域内的文件。** 需要跨文件改动 → 发消息给 owner / Lead，由对方改。
2. **只有 Lead 操作 git index**：不要 `git add/commit/reset/checkout/clean/stash/branch/rm`。
   可以用只读命令：`git status`、`git log`、`git show HEAD:<path>`（读旧实现）。
3. **不新增依赖**（Cargo.toml 只有 Lead 改）；需要依赖 → 发消息给 Lead，写清 crate/版本/features。
4. **不跑全局 `cargo fmt`**（Lead 在集成时统一跑）；可以手工保持格式。
5. `cargo check` 共用 target 目录，可能等锁；按路径过滤自己文件的错误：
   ```bash
   cargo check --all-targets --message-format=short 2>&1 | grep -E '^src/ui/content' | head -60
   ```
6. **绝不臆造 API**：以上游 crate 源码为准（见 docs/gpui-kit-api-notes.md 顶部路径），
   或 `https://gpui-kit.com/component/{name}.md`。
7. 代码注释/技术文档用英文；UI 文案与 PLAN 用中文；日志用 `eprintln!("caw: ...")`（P3 前）。
8. 每个任务完成时：跑自己范围的 check，**明确报告**：改了哪些文件、验证命令与结果、
   剩余风险；然后 `team_task_update(action=complete)` 并给 Lead 发消息。
9. 被阻塞立即发消息，不要空转。共享任务先 `team_task_get` 拿 revision 再 claim/complete。

## 八、验证清单（P4 收口）

- [ ] `cargo check --all-targets` → 0 error
- [ ] `cargo clippy --all-targets` → 0 error
- [ ] `cargo fmt --check` → pass
- [ ] `timeout 20 cargo run` 启动无 panic；日志出现 `caw: scan complete`
- [ ] 手动 UI 冒烟：切 6 个视图 / 搜索 / 播放 / 暂停 / 上下首 / 音量 / 进度 / 歌单增删
- [ ] 主题切换、字体切换、关闭确认、托盘、MPRIS（`playerctl`）可用
- [ ] `grep -ri "tauri\|vue\|npm\|vite" --exclude-dir=target --exclude-dir=.git .` 无残留
- [ ] 文档与实现一致；CI 可复现

## 九、风险

- gpui-kit 0.7 API 与旧示例差异大 → 以 vendored 源码为准，先小步 check。
- 子系统无测试套件 → 以 `cargo check/clippy` + 启动冒烟为主，不编造无意义测试。
- MPRIS/托盘的线程模型与 GPUI 主循环隔离 → 通过 channel + 250ms 轮询接入，禁止在回调里动 Entity。
- 多 agent 并发编译抢锁/写冲突 → 写作用域隔离 + 只有 Lead 提交 + 按文件过滤错误。
- 音频/UI 耦合的 `cx.spawn` 借用错误 → 先 clone 所需数据再 spawn。

## 十、实施记录与决策（持续更新）

### 2026-10-21 / P1 完成

- `cargo check --all-targets` 首次达到 **0 error**（P1 完成）。`src/ui/content.rs` 拆分为
  `src/ui/content/{mod,tracks,artists,albums,playlists,folders,settings,queue}.rs`；
  侧栏与播放栏完成 API 对齐；`main.rs` + `root_view.rs` 完成"PlayerApp 即根视图"重构。
- 新增模块：`config.rs`（AppConfig JSON）、`logging.rs`（文件日志 + 轮转）。

### 关键决策

1. **根视图**：`impl Render for PlayerApp`（`root_view.rs`），删除 `CawWindow` 包装；
   250ms 定时器在 `main.rs` 中用 `WeakEntity` 驱动，窗口关闭时自动退出。
2. **模态框**：歌单创建/加歌用根视图内联遮罩（`app.create_playlist_input` /
   `app.add_to_playlist_track`）；退出确认用 gpui-kit `window.open_alert_dialog`
   （`ui/dialogs.rs`），关闭行为由 `AppConfig.close_to_tray` 决定（最小化 vs 确认退出）。
3. **系统托盘暂缓**：`ksni 0.3.7` 依赖 zbus 5.19 已不再生成的 `DBUS_CONTEXT` task-local，
   实测无法编译（~37 个错误，全部在 ksni 内部）。不在 0.4.0 引入托盘；MPRIS 用 zbus 直连
   实现。后续可手写 StatusNotifierItem 或等 ksni 修复。
4. **zbus 依赖**：仅 Linux 目标依赖 `zbus = { version = "5", features = ["blocking-api"] }`，
   避免影响 Windows CI。
5. **目录选择器**：不引入 `rfd`（GTK/xdg-portal 依赖与主循环问题）；设置页用应用内路径输入
   对话框（`window.open_dialog` + `InputState`）。
6. **窗口状态**：由 `platform::window_state` 独立持久化到 `~/.config/caw/window.json`
   （原子 tmp+rename），不阻塞 `config.rs`。
7. **封面加载**：运行时封面一律 `img(PathBuf)`；`String` 会走 AssetSource 而加载失败。

### 已知遗留（0.4.0 之后）

- 系统托盘（见决策 3）。
- 队列拖拽排序 / 播放历史 UI 的完整交互（core 提供数据 API，content 渲染）。
- 表格列宽拖拽、封面主色衍生背景等 UI-POLISH P2 项。
- 无自动化测试；以 `cargo check/clippy` + 启动冒烟为主。

## 十一、交付验收结果（2026-10-21）

### 自动化验收（全部通过）

```text
cargo check --all-targets      -> 0 errors, 0 warnings (仅 ~/.cargo 配置提示)
cargo clippy --all-targets     -> 0 errors, 0 code warnings
cargo fmt --check              -> pass
cargo test --all-targets       -> 5 passed (config round-trip / corrupt defaults / logging rotation)
```

### 运行时验收（DISPLAY=:1 / Wayland 实测）

- 启动成功：窗口渲染、曲库扫描（193 首）、播放/暂停/上下首/进度/音量全部可用。
- 会话恢复：重启后恢复上次曲目与 117.4s 位置（暂停态），日志确认 restored session at 117.4s。
- MPRIS 实测（busctl）：org.mpris.MediaPlayer2.caw 注册成功；Identity=Caw、PlaybackStatus、
  Metadata（含 artUrl）、Position 正常；Play/Pause/Next/Previous/Seek/Volume/Raise 均生效；
  播放位置随音频推进（144.0s -> 147.0s / 3s）。
- 主题：config.json theme_mode=light 启动即为亮色；恢复 dark 后为暗色（截图验证）。
- 设置页：外观（亮/暗）、字体（Noto Sans CJK SC / Noto Sans / DejaVu Sans）、行为（退出/最小化）、
  音乐文件夹（增删/重扫）渲染正常。
- 日志：~/.config/caw/caw.log 2MiB x 5 轮转，控制台保留 caw: 前缀。

### 本阶段额外完成（超出 P1 的 P2/P3 项）

- 设置：主题/字体/关闭行为 + 应用内路径对话框（无 rfd 依赖）。
- 封面：确定性渐变占位（专辑/艺术家/详情）；运行时封面走 img(PathBuf)。
- 队列抽屉：保存队列为播放列表、最近播放历史、清空、跳播。
- 曲目表：预计算 Arc 队列（去掉 O(n^2) 克隆）+ 行内“添加到播放列表”。
- 歌单删除二次确认（AlertDialog）、退出确认、窗口几何持久化（window.json）。
- 表格/播放栏/侧栏扫码状态、空状态、跑马灯标题。

### 明确遗留（下一版本）

- 系统托盘（ksni 与 zbus 5.19 不兼容；需手写 StatusNotifierItem 或等上游修复）。
- 队列拖拽排序、播放列表内“从歌单移除”的独立入口。
- 表格列宽拖拽、封面主色衍生详情页背景、托盘/媒体键图标资源完善。
- 打包：CI 目前产出 tar.gz/zip；deb/rpm/AppImage 需要额外的打包工具链。
- 无自动化 UI 测试（GUI 以启动 + MPRIS + 截图冒烟为准）。

### 迁移结论

Tauri v2 + Vue 3 + npm 已全部移除；仓库为单一 Rust crate。文档中的 Tauri 字样仅出现在历史说明。
后续开发遵循 AGENTS.md 与 docs/gpui-kit-api-notes.md。

### 热修复：所有视图无法滚动（2026-10-21）

- 根因：内容视图根节点 `v_flex().size_full()` 把高度强制为滚动视口高度，GPUI 计算
  `content_size` 时得到与视口相同的尺寸，`scroll_max = 0`，滚轮事件被忽略（内容只是被裁剪）。
- 修复：`src/ui/content/*.rs` 的视图根节点改为 `.w_full()`（自然高度），滚动容器
  （`.id("content-scroll").overflow_y_scroll()`）即可得到真实内容高度；8 处替换。
- 验证：重启后窗口可滚动到第 11–28 行（截图确认滚动偏移生效），队列抽屉/设置页同理。

### 设置字体选择改为系统字体下拉搜索（2026-10-21）

- 设置页字体不再硬编码三个按钮，改为可搜索下拉框（gpui-kit `Combobox` + `SearchableVec`）。
- 字体列表来自 `cx.text_system().all_font_names()`（本机枚举到 534 个 family），去重排序；
  当前配置字体若不在列表中会补到首位，保证触发器始终显示已选值。
- 选择后立即 `Theme.font_family` 生效并写入 `config.json`（事件：Change/Confirm）。

### 系统托盘 + 双击播放 + 正在播放高亮（2026-10-21）

- **托盘已实现**：`src/platform/tray.rs` 直接基于 zbus 5 实现 StatusNotifierItem +
  com.canonical.dbusmenu（ksni 与 zbus 5.19 不兼容，改为手写）。KDE 的
  StatusNotifierWatcher 已注册成功；菜单：显示 Caw / 播放·暂停 / 下一首 / 上一首 / 退出；
  左键 Activate 显示窗口，滚轮调音量；无托盘宿主时降级为仅最小化并记录警告。
- 关闭按钮在 `close_to_tray=true` 时保存配置并最小化窗口，托盘“显示 Caw”恢复窗口。
  标题栏自带的“—”最小化按钮由窗口管理器处理，GPUI 没有拦截 API，仍为普通最小化；
  托盘图标常驻，可随时恢复。
- **双击播放**：全部音乐/歌单/专辑/艺术家共用曲目表，行双击（`click_count >= 2`）
  以当前列表为队列并跳到该行播放；单击仍只选中。
- **正在播放高亮**：当前行改为 `primary` 14% 透明底色 + 2px 左侧强调条 + 主色标题，
  暂停时仍保持高亮（当前曲目而非仅“播放中”）。

### 音乐文件夹改用系统原生选择器（rfd，2026-10-21）

- 之前决策 5（不用 rfd、用应用内路径输入）已取消：设置页“添加文件夹…”改为
  `rfd::AsyncFileDialog::pick_folder()`，Linux 走 xdg-desktop-portal（KDE 原生对话框），
  不引入 GTK 主循环；在其他平台使用系统原生对话框。
- 代码：`PlayerApp::pick_music_folder` 在 GPUI 协程里 await 对话框，选中后
  `add_music_dir` → 重新扫描；取消则无操作。
- 依赖：`rfd = "0.17"`（默认启用 xdg-portal + pollster，不拉 tokio/async-std）。

### 滚动条可见性修复（2026-10-21）

- 根因：内容区和队列抽屉只用了 `overflow_y_scroll()`（纯裁剪 + 滚轮），没有挂
  gpui-component 的 `Scrollbar` 组件，因此永远看不到滚动条；主题默认 `ScrollbarMode::Scrolling`
  也只在滚动瞬间短暂显示。
- 修复：
  1. 内容区与队列抽屉改用 `ScrollableElement::overflow_y_scrollbar()`（保留滚动容器本身，
     叠加 `Scrollbar` 覆盖层并自动 `track_scroll`）；
  2. 启动时设置 `Theme::set_scrollbar_mode(ScrollbarMode::Always, cx)`，滚动条常驻，
     侧栏列表/下拉列表/对话框等所有使用组件滚动条的区域一并受益。
- 验证：设置页、全部音乐均可看到右侧滚动条（截图），滚轮仍正常；`cargo check/clippy/fmt/test` 全绿。

### 设置选项选中态不显示（2026-10-21）

- 根因：`Button::toggled(bool)` 在 gpui-component 0.7 里**只设置无障碍 aria-toggled**，
  不绘制选中样式（源码：`toggled` 仅写入 `aria_toggled`）；视觉选中要走 `Selectable::selected`
  或换变体。因此“亮色/暗色”“直接退出/最小化到托盘”点击后看起来没反应。
- 修复：设置页新增 `choice_button(id, label, active)` 帮助函数——选中项用 `primary()`（实心），
  未选中用 `outline()`，并额外 `.selected(true/false)` 提供状态样式；播放栏的模式/队列按钮
  同步加 `.selected(...)`（保留 `.toggled` 供读屏器）。
- 配置读写本身正常（`set_close_to_tray` → `config.json` → 关闭钩子读取），只是此前没有视觉反馈。

### 播放栏“定位到正在播放”（2026-10-21）

- 播放栏右侧新增十字准星按钮（`assets/icons/locate.svg`，`CawIcon::Locate`），
  点击后把当前视图滚动到正在播放的曲目并保留其高亮；无当前曲目时按钮禁用。
- 实现：`PlayerApp.content_scroll_handle`（`ScrollHandle`）由内容区 `track_scroll` +
  `vertical_scrollbar` 共享；`locate_now_playing(window, cx)` 计算当前视图里当前曲目的行号，
  `set_offset` 到 `row * LIST_ROW_HEIGHT - 1.5 行`（GPUI 在 prepaint 里会 clamp）。
- 曲目表行高固定为 `LIST_ROW_HEIGHT = 38px`，否则像素定位会漂移。
- 视图自适应：艺术家/专辑网格 → 打开当前曲目的详情；文件夹 → 打开所在目录；
  播放列表列表/设置 → 回到“全部音乐”；搜索过滤把当前曲目隐藏时会自动清空搜索。
- 实测：日志 `locate: track ... at row 19, scroll offset 665px`，截图确认第 20 行高亮并滚入视野。

### 浮动“正在播放”按钮 + 点击歌名定位（2026-10-21）

- 当正在播放行滚出内容视口时，右下角（播放栏上方）浮出胶囊按钮「◎ 正在播放」，
  点击立即滚回当前曲目；行可见时自动隐藏。
- 可见性判断：`PlayerApp::now_playing_row_visible()` 用内容区 `ScrollHandle` 的
  offset/bounds 与当前行几何比较；行几何由每个视图固定的“首行顶部 + 行距”常量给出
  （AllMusic 138px、详情 154px、文件夹 104px/44px 行距），所以视图头部高度改为固定值
  （AllMusic 工具栏 56px，详情头 72px）。
- 播放栏左下角的歌名/歌手区域整块可点击，同样触发定位（cursor 变手型）。
- 播放栏右下角的小十字准星按钮保留，作为不滚动时的显式入口。

### 发行打包：deb / rpm / AppImage（2026-10-21）

- 新增 `scripts/package.sh [deb|rpm|appimage|tarball|all]`，从 `cargo build --release --locked`
  的产物生成四种发行物：
  - `.deb`：`dpkg-deb --root-owner-group`，带 control（Depends: ALSA/xkbcommon/wayland/xcb/
    fontconfig/freetype/openssl/vulkan）与 /usr/share 桌面文件、图标、文档；
  - `.rpm`：用 `rpmbuild` + 生成的 spec（Fedora 的 `rpm-build` 提供）；
  - `.AppImage`：AppDir + `AppRun` + `appimagetool`（支持 `APPIMAGETOOL` 覆盖，
    自动尝试 `--appimage-extract-and-run` 以规避 FUSE）；
  - 便携 tar.gz + sha256。
- 新增 `packaging/caw.desktop`、`packaging/AppRun`（AppImage 入口），复用
  `assets/icons/music.svg` 作为应用图标。
- `.github/workflows/build-rpm.yml` 的 Linux job 安装 `rpm` 与 appimagetool 后执行
  `./scripts/package.sh all`，`dist/*` 全部上传并在 tag 时附加到 Release。
- 本地验证：deb（12MB，control/文件清单正确）、AppImage（17MB，可解包）、tar.gz 均构建成功；
  本机未安装 rpm-build，rpm 由 CI 覆盖。

### 打包验证补充（2026-10-21）

- `SKIP_BUILD=1` 支持复用已编译的 `target/release/caw`（容器/CI 复用构建产物）。
- 在 `fedora:latest` 容器内用 `rpm-build` 实测成功：`caw-0.4.0-1.x86_64.rpm`（13MB），
  `rpm -qip/-qlp` 元数据与文件清单正确（/usr/bin/caw、desktop、图标）。
- `.deb`（12MB）元数据与清单正确；`.AppImage`（17MB）实测可启动（MPRIS/托盘/扫描均正常）；
  `tar.gz` + sha256 正常。

### 修复：选择“最小化到托盘”后点关闭仍然退出（2026-10-21）

- 根因：系统层关闭请求（窗口管理器 X / Alt+F4 / 会话关闭，经 `xdg_toplevel::Close`）
  不经过 GPUI `TitleBar::on_close_window`，平台默认直接销毁窗口；而之前的关闭偏好只在
  title bar 回调里判断，所以这条路径直接退出。
- 修复：`main.rs` 创建窗口后注册 `window.on_window_should_close(...)`（Wayland/X11 后端均实现），
  回调里读取 `close_to_tray`：
  - true → 保存窗口几何 + flush 配置 + `minimize_window()`，返回 `false`（阻止关闭）；
  - false → 打开退出确认对话框，返回 `false`（取消则保留窗口，确认才 `cx.quit()`）。
- 同时 `set_close_to_tray` 现在立即同步写 `config.json`（不再依赖 750ms 节流），
  避免“刚选中就关窗口”读到旧值。两条路径都加了 `caw:` 日志。
- 验证：用 KWin 脚本 `w.closeWindow()` 触发真实平台关闭——
  `close_to_tray=true` 时日志 `window close requested (close_to_tray=true)` 且进程存活（最小化）；
  `false` 时弹出“退出 Caw？”确认框且进程存活。

### 最小化到托盘时移除任务栏条目（2026-10-21）

- 问题：Wayland 没有“从任务栏移除窗口”的标准协议；GPUI 也没有 hide/unmap API，
  仅 `minimize_window()` 会保留任务栏条目。
- 方案（KDE/Plasma）：`src/platform/kwin.rs` 通过 KWin 的脚本 D-Bus 接口
  （`org.kde.kwin.Scripting.loadScript/start/unloadScript`）在独立线程里执行一段 JS，
  对 `pid == 当前进程` 的窗口设置 `skipTaskbar` / `skipPager`；恢复时再置回并
  `minimized = false` + `workspace.activeWindow = w`。
- 关闭到托盘路径（titlebar 与 `on_window_should_close`）调用 `set_taskbar_visible(false)`；
  托盘“显示 Caw”/MPRIS Raise 调 `set_taskbar_visible(true)` 并 activate。
- 非 KDE 桌面：该函数直接返回（GNOME 等 Wayland 合成器没有等价机制），退化为最小化；
  这是平台限制，已在 README 说明。
- 验证：KWin 查询脚本确认 `skipTaskbar=true/minimized=true`，任务栏截图中 Caw 图标消失；
  恢复后 `skipTaskbar=false/minimized=false` 且窗口重新出现在任务栏并激活。

### 托盘菜单美化：音乐播放器风格（2026-10-21）

- 菜单结构改为：
  1. “正在播放”信息行（禁用态）：`歌名 — 歌手  当前/总时长`，图标随播放状态变化；
  2. 分隔线；
  3. 播放/暂停（图标 `media-playback-*-symbolic`，文案随状态变化）、上一首、下一首；
  4. 分隔线；
  5. “音量  N%”子菜单：0/25/50/75/100% 单选（radio，当前档位勾选），点击即调音量；
  6. “播放模式 · 顺序播放”子菜单：顺序/随机/列表循环/单曲循环 单选，点击切换；
  7. 分隔线；
  8. “显示 Caw”“退出 Caw”。
- DBusMenu 属性使用 `icon-name`（freedesktop symbolic 图标）、`toggle-type/toggle-state`
  单选状态、`children-display=submenu`；`GetLayout` 返回根据播放状态（曲目/时长/音量/模式/
  播放状态）计算出的 revision，客户端重新拉取时能刷新文案与勾选。
- 新增 `PlatformCommand::SetPlaybackMode` 与 `PlayerApp::set_playback_mode`，
  托盘菜单切换模式与主界面/配置保持同步。
- 验证：`GetLayout` 输出包含信息行/图标/子菜单与勾选状态；菜单 Event 实测
  50% 音量 → MPRIS Volume=0.5，随机 → Shuffle=true，单曲循环 → LoopStatus=Track。

### 修复：退出应用不保存播放进度（2026-10-21）

- 根因：`persist_session()` 只在“开始播放 / 暂停 / seek / 切歌”时调用，播放过程中
  `config.last_position` 一直停留在上一次事件的值；正常退出依赖 `Drop` 再保存，
  但如果退出路径没有走到 Drop（或在写盘节流窗口内），位置就会丢失，重启即 0。
- 修复：
  1. 播放中每 5 秒调用 `save_state_now()`（同步原子写 config.json），
     即使崩溃/被 kill 也只丢最近几秒；
  2. 退出相关路径改为**同步**保存，不再依赖节流或 Drop：托盘/MPRIS Quit、
     退出确认框“退出”、关闭到托盘（最小化前）都调用 `save_state_now()`；
  3. `Drop for PlayerApp` 仍保留最终同步保存兜底。
- 验证（真实进程）：播放到 74.6s → MPRIS Quit → 进程退出、config.last_position=74.96；
  重启日志 `restored session at 75.0s`，MPRIS Position≈75s；受控测试写入 42.0s 后启动，
  状态为 Paused、Position 恰好 42.0s。

### 修复：托盘“音量/播放模式”悬浮弹出重复菜单（2026-10-21）

- 根因：DBusMenu 的 `GetLayout` 忽略了 `parent_id`，客户端请求子菜单（音量 id=7、
  播放模式 id=8）的布局时仍返回整棵根菜单，Plasma 于是在旁边渲染出第二份菜单。
- 修复：
  1. `GetLayout(parent_id)` 按父项返回对应子菜单（7 → 音量档位，8 → 播放模式），
     其他未知 id 返回空布局；
  2. 尊重 `recursion_depth`：depth=1 只返回直接子项、不内联子菜单内容（避免同一批
     子项既嵌套在根布局里又被单独请求）；depth<0 返回完整树；
  3. “显示 Caw”图标换成 Breeze 存在的 `view-visible-symbolic`（原来的
     `window-restore-symbolic` 显示为空白方块）。
- 验证：`GetLayout(0, depth=1)` 只含 8 个顶层项；`GetLayout(7)` 返回 0/25/50/75/100%；
  `GetLayout(8)` 返回四种播放模式。

### 修复：托盘音量显示/勾选不准（2026-10-21）

- 现象：父项显示“音量 0%”，子菜单却勾选 50%；实际音量 38%。原因有两个：
  1. 子菜单把 0/25/50/75/100% 做成 radio，并用“最接近档位”判断勾选，38% 会被算成 50%，
     看起来像音量错了；
  2. 菜单文案是动态的，但从未向客户端发 `LayoutUpdated`，Plasma 缓存了旧布局（0%）。
- 修复：
  - 音量子菜单改为：`当前音量 N%`（禁用信息行）+ `音量 +5%` / `音量 -5%` + `静音`（check 状态）
    + 0/25/50/75/100% 预设（纯动作，不再有 radio/勾选）；
  - 父项与信息行直接显示真实百分比；
  - 托盘线程在菜单相关内容（曲目、播放状态、音量、模式、时长、进度 5s 粒度）变化时
    发出 `LayoutUpdated(revision, parent=0/7/8)`，客户端会刷新标签与勾选；
  - 顺带 `set_volume` 已写入 `config.volume`，音量现在跨重启保留。
- 验证：`GetLayout(7)` 无 radio，含“当前音量 38%”与 ±5%/静音/预设；把音量改成 62% 时
  dbus-monitor 捕获 3 条 LayoutUpdated，根布局立即显示“音量 62%”。

### 发布 0.5.0（2026-10-21）

- 版本从 0.4.0 升到 **0.5.0**，覆盖本轮功能：Tauri→gpui-kit 迁移收尾、系统托盘
  （SNI + DBusMenu，含 KDE 任务栏隐藏）、音乐播放器风格菜单（正在播放/音量/模式子菜单）、
  搜索式系统字体选择、双击播放/正在播放高亮、定位正在播放（浮动按钮 + 点歌名）、
  会话恢复与退出保存进度、滚动条常驻、rfd 原生目录选择、deb/rpm/AppImage 打包脚本。
- 产物：`dist/caw_0.5.0_amd64.deb`（12MB，`dpkg-deb -f` 校验 Name=0.5.0/amd64 通过）。

### 新应用图标 + 跨平台编译修复（2026-10-21）

- 采用 `assets/icons/caw.svg` 作为应用图标：去掉白色底矩形（透明背景），并生成
  `assets/icons/caw.png`（256x256，含 alpha）供系统托盘使用。
- 托盘不再依赖主题图标名：`StatusNotifierItem.icon-name` 置空，改用内嵌 PNG 生成的
  `icon-pixmap`（24/32/48/64，ARGB32），KDE 图标缓存未刷新时也不会显示“?”占位。
- 打包（deb/rpm/AppImage/tarball）同时安装 `caw.svg`（scalable）与 `caw.png`（256x256），
  `scripts/dev-setup.sh` 同样安装两种尺寸。
- 修复 Windows 编译：`platform::set_taskbar_visible()` 包装 KWin 专用调用，非 Linux 为 no-op；
  CI 中 Windows job 标记 `continue-on-error`，Release 只依赖 Linux job，避免 Windows 失败
  阻断 Linux 产物发布。

### 应用图标在窗口/任务栏/应用内生效（2026-10-21）

- 托盘：使用内嵌 PNG 生成的 `icon-pixmap`（透明底，24/32/48/64），`icon-name` 置空，
  不再出现白色方块或“?”占位。
- 窗口/任务栏：`WindowOptions.app_id = "caw"` + `titlebar.title = "Caw"`，KWin 查询确认
  `desktopFileName=caw`、`caption=Caw`，compositor 因此能匹配 `caw.desktop` 并使用新图标；
  dev-setup 安装的 desktop 文件也从 `cn.cyber-nest.caw.desktop` 改为 `caw.desktop`（与 app_id 一致）。
- 应用内：标题栏与侧栏 logo 改用 `CawIcon::App`（`assets/icons/caw.svg`），放在白色圆角底板上
  保证深浅主题下都清晰。

### 0.5.1：应用内改用简化图标（2026-10-21）

- 详细插画（800x600 黑胶+几何图形）在 16–24px 的标题栏/侧栏里会糊成一团，
  因此新增 `assets/icons/caw-mark.svg`：极简黑胶唱片（外圈 + 两道纹路 + 白色中心孔），
  16/24px 下依然清晰；标题栏与侧栏 logo 改用它，仍放在白色圆角底板上。
- 托盘 `icon-pixmap` 也改用 `caw-mark.png`（256px 透明底）生成，小尺寸更清楚；
  桌面/任务栏/安装包图标继续使用详细的 `caw.svg`/`caw.png`。
- 版本号升到 **0.5.1**（About 同步）。
