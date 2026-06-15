# Caw — 项目说明 (Project Instructions)

本地音乐播放器，纯 Rust 桌面应用。GPU 加速 GUI（GPUI）+ 音频解码/输出（Symphonia / cpal）。
详细架构与分阶段实施计划见 `PLAN.md`（中文，权威文档）。

> A local music player. Pure-Rust desktop app: GPU-accelerated GUI (GPUI) plus audio
> decode/output (Symphonia / cpal). Architecture and the phased roadmap live in
> `PLAN.md` (in Chinese) — treat it as the source of truth.

## 项目本质 (What this is)

- **语言**: Rust, edition 2024, toolchain ~1.96.
- **GUI 框架**: `gpui`（Zed 的 GPU 渲染框架）+ `gpui-component`（longbridge 的 shadcn 风格组件库）。
- **音频**: `symphonia`（解码 FLAC/MP3/WAV/Ogg/AAC/ALAC…）→ `cpal`（Linux 走 ALSA/PipeWire 输出）。
- **关键依赖均为 git 依赖**（跟随 `zed-industries/zed` 与 `longbridge/gpui-component` 的 main 分支），
  **不是** crates.io 发布版。因此 API 可能随时变化——改动 UI 时优先看 `target/` 里已拉取的源码，
  或对照上游仓库，切勿凭记忆假设 API。
- **目标平台**: Linux（开发机为 Fedora）。`build.rs` 里有针对 Fedora 的 `libxkbcommon-x11.so.0`
  符号链接 hack；字体设为 `Noto Sans CJK SC`。改动窗口/主题/字体相关代码时注意这套 Linux 适配。

## 常用命令 (Build / run / check)

```bash
cargo run                 # 编译并运行（首次因 git 依赖会较慢，之后增量）
cargo build               # 仅编译
cargo check               # 快速类型检查（改完代码先跑这个）
cargo clippy --all-targets  # 风格/lint 检查（建议提交前跑）
cargo fmt                 # 格式化
# 无测试套件（目前未编写测试）
```

- **改完任何代码后先跑 `cargo check`**；改动较大或涉及 audio/player 时再 `cargo run` 实际验证。
- 这是个 GUI 应用，`cargo test` 目前没有用例；不要为了让“测试通过”而编造无意义测试。

## 架构与约定 (Architecture & conventions)

模块布局（见 `src/`）：

- `main.rs` — 入口：gpui 应用初始化、暗色主题调色板、窗口创建、`PlayerApp` Entity 创建、
  **后台异步扫描曲库**（`cx.spawn`，窗口先打开、扫描完成后再更新 UI）。也是全局键盘快捷键
  （空格=播放/暂停、←/→=上下首、↑/↓=音量）的挂载点。
- `app.rs` — `PlayerApp`：跨视图共享的顶层状态（`Entity<PlayerApp>`）。曲库 `Vec<Arc<Track>>`、
  索引式过滤视图 `FilteredView`、播放状态、音量、shuffle/repeat、当前 nav 选中项等都在这里。
- `ui/` — `sidebar.rs`（左导航）、`content.rs`（中部专辑网格/歌曲表）、`player_bar.rs`（底部控制栏）。
- `audio/` — `library.rs`（walkdir 扫描）、`decoder.rs`（Symphonia 解码封装）、
  `player.rs`（cpal 输出引擎 + `PlayerCommand` 跨线程指令）。
- `models/` — `track.rs`（`Track` 元数据，`AudioFormat`）、`playlist.rs`（`PlaybackState`/`RepeatMode`）。

### 必须遵守的约定

- **状态即 Entity**：所有可变状态用 GPUI 的 `Entity<T>`（`cx.new(...)`）管理；跨视图共享一律传
  `Entity<PlayerApp>`（或其 `clone()`）。状态变化后 **必须 `cx.notify()`** 触发重绘，否则 UI 不更新。
- **音频线程隔离**：解码与 cpal 输出在独立线程，UI 线程绝不能阻塞。跨线程用 `crossbeam_channel`
  传 PCM、用 `AtomicBool`/`AtomicUsize` 共享 playing/seek 等标志；**cpal 回调里禁止阻塞或重锁**。
- **进度更新**：播放位置由音频线程推进，UI 通过定时器（`cx.spawn` + `Timer::interval`，约 250ms）
  轮询并 `cx.notify()`，不要在音频回调里直接动 Entity。
- **不要复制 `Vec<Track>`**：大列表用 `Arc<Track>` 和索引视图（`FilteredView::indices`），
  避免每次 render 全量克隆。
- **错误处理**：单文件解码失败就跳过并记录（`eprintln!("caw: ...")`）；播放中途出错自动跳下一首；
  不要因为个别坏文件让整个扫描/曲库崩掉。
- **日志**：调试/状态信息用 `eprintln!("caw: ...")` 前缀，便于在终端里 grep。

## 文档/注释风格

- 代码注释、commit message、技术文档用**英文**（与现有代码一致）。
- `PLAN.md` 及面向用户的说明用**中文**。回答用户用中文。
- 回答尽量简洁，先给结论/命令，再按需补充解释。
