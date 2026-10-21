# Caw

> 本地音乐播放器 · 纯 Rust · GPUI (gpui-kit) 界面

[![Build](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml/badge.svg)](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml)

Caw 是一个本地音乐播放器。整个应用是**单个 Rust crate**：没有 Node、没有 WebView、
没有前端构建链。界面由 [gpui-kit](https://gpui-kit.com)（Zed 的 GPUI + gpui-component）
GPU 加速渲染，音频由 Symphonia 解码、cpal 输出。

## 功能

- 🎵 **多格式支持** — FLAC / MP3 / WAV / Ogg / AAC / ALAC（Symphonia 解码）
- 📂 **多目录音乐库** — 支持添加多个音乐文件夹，按规范路径去重，后台扫描不卡 UI
- 🎨 **GPU 界面** — 自定义无边框标题栏、亮/暗主题、字体切换、暗色配色
- 📋 **播放队列** — 队列抽屉、跳播、清空；（拖拽排序/历史待办，见 PLAN.md）
- 🔀 **播放模式** — 顺序 / 随机 / 列表循环 / 单曲循环
- 📻 **持久化播放列表** — 基于本地 SQLite 安全存储，支持创建/删除/加歌/移除
- 🔍 **搜索与过滤** — 按标题 / 艺术家 / 专辑即时过滤
- 📊 **多维资料库视图** — 全部音乐 / 艺术家 / 专辑 / 文件夹 / 播放列表 / 设置
- 💾 **会话恢复** — 记住音量、播放模式、上次播放的曲目/队列/位置（暂停态恢复）
- 📻 **MPRIS 集成** — Linux 桌面媒体键、播放小部件控制（zbus）
- ⌨️ **键盘快捷键** — 空格播放/暂停，←/→ 上下首，↑/↓ 音量（输入时自动让位）

> 系统托盘暂缓：`ksni 0.3.7` 与 `zbus 5.19` 不兼容，托盘支持推迟到后续版本，
> 详见 `PLAN.md`。

## 技术栈

| 层级 | 技术 | 说明 |
|---|---|---|
| **GUI** | [gpui-kit](https://gpui-kit.com) 0.7（GPUI + gpui-component） | 单一 Rust GUI 依赖，GPU 渲染 |
| **音频解码** | [Symphonia](https://github.com/pdeljanov/Symphonia) | 纯 Rust 多格式解码 |
| **音频输出** | [cpal](https://github.com/RustAudio/cpal) | Linux: ALSA / PipeWire |
| **本地存储** | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite)) | 播放列表、音乐目录 |
| **偏好配置** | `~/.config/caw/config.json`（serde） | 主题、字体、音量、会话恢复 |
| **系统集成** | [zbus](https://github.com/dbus2/zbus) | Linux MPRIS（媒体键/小部件） |

## 开发

### 系统依赖（Fedora）

```bash
sudo dnf install alsa-lib-devel libxkbcommon-devel libxkbcommon-x11 \
     wayland-devel libX11-devel fontconfig-devel freetype-devel \
     vulkan-loader-devel openssl-devel
```

### 命令

```bash
cargo run                       # 编译并运行
cargo check --all-targets       # 快速类型检查
cargo clippy --all-targets      # lint（提交前）
cargo fmt                       # 格式化
cargo build --release           # 发布构建（target/release/caw）
```

可选：`scripts/dev-setup.sh` 会安装一个 hicolor 图标与 `.desktop` 文件，
让 Wayland 下的窗口在任务栏显示图标。

### 运行时数据

全部位于 `~/.config/caw/`：

| 文件 | 用途 |
|---|---|
| `config.json` | 偏好与上次播放会话 |
| `caw.db` | SQLite：歌单、音乐目录 |
| `caw.log`（及 `.1`…`.5`） | 日志，2 MiB 轮转，控制台保留 `caw: ` 前缀 |
| `covers/` | 从音频标签提取的封面缓存 |

## 快捷键

| 按键 | 行为 |
|---|---|
| Space | 播放 / 暂停 |
| ← / → | 上一首 / 下一首 |
| ↑ / ↓ | 音量 +5% / -5% |

输入框聚焦或对话框打开时，以上快捷键不会抢占按键。

## 构建发布

CI（`.github/workflows/build-rpm.yml`）在 Linux 与 Windows 上执行 `cargo build --release --locked`：
Linux 产出 `caw-linux-x86_64.tar.gz`（含二进制、README、PLAN、图标），Windows 产出 `caw-windows-x86_64.zip`；
打 `v*` tag 时自动附加到 GitHub Release。

## 文档

- `PLAN.md` — 迁移计划、分阶段任务、验收标准（权威）
- `AGENTS.md` — 仓库约定（模块、API、音频线程、文档风格）
- `CONTEXT.md` — 领域术语表
- `docs/gpui-kit-api-notes.md` — gpui-kit 0.7 已验证 API 速查

## 许可

本项目采用 MIT License 开源许可。
