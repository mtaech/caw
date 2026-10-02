# Caw

> 本地音乐播放器 · 纯 Rust 音频引擎 + Vue 3 界面

[![Build](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml/badge.svg)](https://github.com/mtaech/caw/actions/workflows/build-rpm.yml)

## 功能

- 🎵 **多格式支持** — FLAC / MP3 / WAV / Ogg / AAC / ALAC（Symphonia 解码）
- 📂 **多目录音乐库** — 支持添加多个音乐文件夹，自动去重与后台快速扫描
- 🎨 **现代界面** — GPU 加速渲染，深色/浅色/跟随系统主题，自定义无边框标题栏与字体切换
- 📋 **播放队列** — 拖拽排序、播放历史、保存为播放列表、支持队列替换与插入模式
- 🔀 **播放模式** — 顺序 / 随机 / 列表循环 / 单曲循环
- 📻 **持久化播放列表** — 基于本地 SQLite 数据库安全存储
- 🔍 **搜索与过滤** — 毫秒级按标题 / 艺人 / 专辑即时过滤
- 📊 **多维资料库视图** — 全部音乐 / 艺术家 / 专辑 / 文件夹目录树
- 🔈 **MPRIS 集成** — Linux 系统媒体键、桌面播放小部件及锁屏控制集成
- 📌 **系统托盘** — 最小化到托盘后台持续播放、关闭前确认提示
- 🖥️ **跨平台支持** — Linux + Windows

## 技术栈

| 层级 | 核心技术 | 说明 |
|---|---|---|
| **应用框架** | [Tauri v2](https://v2.tauri.app/) | 轻量级跨平台桌面框架 |
| **音频解码** | [Symphonia](https://github.com/pdeljanov/Symphonia) | 纯 Rust 高保真多格式解码器 |
| **音频输出** | [cpal](https://github.com/RustAudio/cpal) | 底层低延迟音频输出（Linux: ALSA / PipeWire） |
| **前端架构** | Vue 3 + TypeScript + Pinia | 响应式状态流与组合式 API |
| **样式与组件** | Tailwind CSS + [Radix Vue](https://www.radix-vue.com/) + [Lucide](https://lucide.dev/) | 现代化暗色设计系统 |
| **本地存储** | SQLite ([rusqlite](https://github.com/rusqlite/rusqlite)) + Tauri Store | 播放列表与偏好配置持久化 |
| **系统集成** | [zbus](https://github.com/dbus2/zbus) (MPRIS) | Linux D-Bus 媒体总线集成 |

## 开发

```bash
# 安装前端依赖
npm install

# 启动开发模式（前端热重载 + 后端编译）
npm run tauri dev

# 仅前端类型检查
npx vue-tsc --noEmit

# 仅构建前端产物
npm run build

# 仅后端类型检查
cd src-tauri && cargo check
```

## 构建打包

```bash
npx tauri build --bundles rpm       # Linux RPM 包
npx tauri build --bundles deb       # Linux DEB 包
npx tauri build --bundles appimage  # Linux AppImage
npx tauri build --bundles nsis      # Windows 安装包
```

或一次性构建所有目标包：

```bash
npx tauri build
```

## 下载

访问 [GitHub Releases](https://github.com/mtaech/caw/releases) 获取最新的 RPM / DEB / AppImage / Windows 安装包。

## 许可

本项目采用 MIT License 开源许可。
