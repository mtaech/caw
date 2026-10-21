# Caw

> 本地音乐播放器 · 纯 Rust · GPUI (gpui-kit) 界面

[![Build](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml/badge.svg)](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml)

Caw 是一个本地音乐播放器。整个应用是**单个 Rust crate**：没有 Node、没有 WebView、
没有前端构建链。界面由 [gpui-kit](https://gpui-kit.com)（Zed 的 GPUI + gpui-component）
GPU 加速渲染，音频由 Symphonia 解码、cpal 输出。

## 功能

- 🎵 **多格式支持** — FLAC / MP3 / WAV / Ogg / AAC / ALAC（Symphonia 解码）
- 📂 **多目录音乐库** — 系统原生文件夹选择器（xdg-desktop-portal），多目录去重，后台扫描不卡 UI
- 🎨 **GPU 界面** — 自定义无边框标题栏、亮/暗主题、字体切换、暗色配色
- 📋 **播放队列** — 队列抽屉、跳播、清空；（拖拽排序/历史待办，见 PLAN.md）
- 🔀 **播放模式** — 顺序 / 随机 / 列表循环 / 单曲循环
- 📻 **持久化播放列表** — 基于本地 SQLite 安全存储，支持创建/删除/加歌/移除
- 🔍 **搜索与过滤** — 按标题 / 艺术家 / 专辑即时过滤
- 📊 **多维资料库视图** — 全部音乐 / 艺术家 / 专辑 / 文件夹 / 播放列表 / 设置
- 💾 **会话恢复** — 记住音量、播放模式、上次播放的曲目/队列/位置（暂停态恢复）
- 📻 **MPRIS 集成** — Linux 桌面媒体键、播放小部件控制（zbus）
- 🖥️ **系统托盘** — StatusNotifierItem + DBusMenu（zbus 手写实现）：显示窗口、播放/暂停、上下首、退出；关闭窗口可最小化到托盘（KDE 下同时从任务栏隐藏，其他 Wayland 桌面仅最小化）
- 🖱️ **双击播放** — 全部音乐 / 歌单 / 专辑 / 艺术家列表双击切歌，当前播放行高亮
- 🎯 **定位正在播放** — 滚出视野时右下角浮出「正在播放」按钮，点击即可滚回；点左下角歌名同样可定位（自动切换视图/清空搜索）
- ⌨️ **键盘快捷键** — 空格播放/暂停，←/→ 上下首，↑/↓ 音量（输入时自动让位）

## 技术栈

| 层级 | 技术 | 说明 |
|---|---|---|
| **GUI** | [gpui-kit](https://gpui-kit.com) 0.7（GPUI + gpui-component） | 单一 Rust GUI 依赖，GPU 渲染 |
| **音频解码** | [Symphonia](https://github.com/pdeljanov/Symphonia) | 纯 Rust 多格式解码 |
| **音频输出** | [cpal](https://github.com/RustAudio/cpal) | Linux: ALSA / PipeWire |
| **本地存储** | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite)) | 播放列表、音乐目录 |
| **偏好配置** | `~/.config/caw/config.json`（serde） | 主题、字体、音量、会话恢复 |
| **系统集成** | [zbus](https://github.com/dbus2/zbus) | Linux MPRIS + 系统托盘（StatusNotifierItem / DBusMenu） |

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

### 本地打包（Linux）

```bash
./scripts/package.sh            # 全部：.deb + .rpm + .AppImage + tar.gz
./scripts/package.sh deb        # 或只打某一种：deb | rpm | appimage | tarball
```

产物输出到 `dist/`：

| 格式 | 产物 | 额外依赖 |
|---|---|---|
| Debian/Ubuntu | `caw_<version>_amd64.deb` | `dpkg-deb`（`dpkg-dev`） |
| Fedora/RHEL | `caw-<version>-1.x86_64.rpm` | `rpmbuild`（Fedora: `rpm-build`；Ubuntu: `rpm`） |
| 通用 | `Caw-<version>-x86_64.AppImage` | [appimagetool](https://github.com/AppImage/appimagetool/releases)（可用 `APPIMAGETOOL=/path` 指定） |
| 便携 | `caw-<version>-linux-x86_64.tar.gz` + `.sha256` | 无 |

缺少某个工具时会跳过对应格式并给出提示；脚本只在实际什么都没打出来时返回非 0。

### CI

`.github/workflows/build-rpm.yml` 在 Linux 上安装 `rpm` 与 `appimagetool` 后执行
`./scripts/package.sh all`，产出 deb / rpm / AppImage / tar.gz；Windows 产出 zip。
打 `v*` tag 时所有产物自动附加到 GitHub Release。

## 文档

- `PLAN.md` — 迁移计划、分阶段任务、验收标准（权威）
- `AGENTS.md` — 仓库约定（模块、API、音频线程、文档风格）
- `CONTEXT.md` — 领域术语表
- `docs/gpui-kit-api-notes.md` — gpui-kit 0.7 已验证 API 速查

## 许可

本项目采用 MIT License 开源许可。
