# Caw

> 本地音乐播放器 · 纯 Rust 音频引擎 + Vue 3 界面

[![Build](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml/badge.svg)](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml)

## 功能

- 🎵 多格式支持 — FLAC / MP3 / WAV / Ogg / AAC / ALAC（Symphonia 解码）
- 📂 多目录音乐库扫描，自动去重
- 🎨 GPU 加速界面，深色主题，自定义标题栏
- 📋 播放队列 — 拖拽排序、播放历史、保存为播放列表
- 🔀 播放模式 — 顺序 / 随机 / 列表循环 / 单曲循环
- 📻 持久化播放列表（SQLite）
- 🔍 搜索 & 过滤 — 按标题/艺人/专辑
- 📊 库视图 — 全部音乐 / 艺术家 / 专辑 / 文件夹
- 🔈 MPRIS 集成（Linux 媒体键 + 桌面环境集成）
- 📌 系统托盘
- 🖥️ Linux + Windows

## 技术栈

| 层 | 技术 |
|---|---|
| 框架 | [Tauri v2](https://v2.tauri.app/) |
| 音频解码 | [Symphonia](https://github.com/pdeljanov/Symphonia) |
| 音频输出 | [cpal](https://github.com/RustAudio/cpal) |
| 前端 | Vue 3 + TypeScript + Pinia + Tailwind CSS |
| 组件 | [Radix Vue](https://www.radix-vue.com/) + [Lucide](https://lucide.dev/) |
| 数据库 | SQLite (rusqlite) |
| D-Bus | zbus (MPRIS) |

## 开发

```bash
# 安装依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 仅类型检查
npx vue-tsc --noEmit

# 仅构建前端
npm run build

# 后端检查
cd src-tauri && cargo check
```

## 构建

```bash
npx tauri build --bundles rpm    # Linux RPM
npx tauri build --bundles deb    # Linux DEB
npx tauri build --bundles appimage # Linux AppImage
npx tauri build --bundles nsis   # Windows 安装包
```

或全部：

```bash
npx tauri build
```

## 下载

[GitHub Releases](https://github.com/mtaech/caw/releases) — 提供 RPM / DEB / AppImage / Windows 安装包。

## 许可

MIT
