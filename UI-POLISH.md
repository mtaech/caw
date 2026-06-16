# Caw 亮色主题美化方案（精修细节）

> 范围决策：**保留现有亮色基调，精修细节**。本方案不含深色主题（仅在设计令牌层
> 为将来留好扩展点）。产出形式：先 review 本文档，再按 P0 → P2 顺序实施。
>
> **P0、P1 已实施完成**（`npm run build` 通过）。P2 待按需推进。
>
> 本文所有判断均基于对 `src/` 全部 18 个组件 + `style.css` + `tailwind.config.js`
> 的逐行审计，并已用 `rg` 验证关键问题点（章节内标注「✅已验证」）。

---

## 一、诊断摘要：当前 UI 有哪些"不对的细节"

整体框架（标题栏 / 侧栏 / 内容区 / 播放栏）是清晰的，问题集中在 **三类**：

1. **真 bug（视觉失效或 jank）**：侧栏激活指示条没垂直居中、表格"+"按钮 hover 永远不触发、
   当前播放行有会撑动布局的多余竖条、`duration-120` 是非法 Tailwind 值（静默失效）。
2. **不一致**：当前播放指示符在 4 个列表视图里各写各的（图标 / `♫` 字符 / 竖条 / 加粗混用）；
   返回导航有的用图标有的用文字；播放主按钮在播放栏是黑色、在详情页是绿色；删除红用了暗色调的红。
3. **视觉"偏碎 / 偏平"**：大量 `border-border` 细分割线 + 卡片边框堆叠，缺少阴影/层级，
   专辑/艺术家无封面时全是重复灰块，封面加载无骨架会闪一下。

---

## 二、设计令牌层（Design Tokens）

目标：把现在散落在各组件里的 ad-hoc 值（红、阴影、z-index、动效时长）收敛成令牌，
后续细节调整只改一处。

### 2.1 颜色

| 令牌 | 现状 | 建议 | 说明 |
|---|---|---|---|
| `--faint-foreground` | `#9999a0`（对比度 ≈2.8:1） | 保留，但**仅用于装饰性文字**（计数、占位） | 任何承载信息的文字不应低于 `--muted-foreground` |
| `--primary` / hover | 深绿 `#1b7937` | 保留 | 白底对比度达 AA，方向正确 |
| **`--destructive`（新增）** | 散落 `red-400/300/500/500-10` | 统一新增 `--destructive` + `--destructive-foreground` | 替换 `PlaylistDetail` / `Settings` / 删除确认框里的裸 red-* |
| **`--ring`（新增）** | Button 用 `ring-foreground`、Input 用 `ring-primary` | 统一为 `--ring`（建议 primary 色） | 聚焦环全局一致 |
| **`--primary-subtle`（新增）** | 现用 `bg-primary/10` 模拟 | 可选：提供一个稳定的浅色背景令牌 | 避免不同底色下透明度叠加发灰 |
| `--table-odd/even` | ✅已验证：**定义了但全项目零引用**（死令牌） | **二选一**：①启用斑马纹（`bg-table-odd` 套在奇数行）；②删除令牌 | 建议启用，提升长表可扫读性 |

### 2.2 字体

- `Noto Sans CJK SC` 设为首选 sans，但**未通过 `@font-face` 引入**，依赖系统安装。
  Fedora 上通常有，但需在文档注明假设；若要稳定可自托管（如拉丁文配 Inter，CJK 仍走系统 Noto）。
- `src/assets/fonts/MapleMono-NF-CN-Regular.ttf` **存在但全项目未被 `@font-face` 引用**（mono 字体形同摆设）。
  决策：要么接上（用于数字/时长等 tabular 场景），要么删除资源避免误导。
- 统一一个 **"overline" 工具类**（`text-caption uppercase tracking-wider`）替代现在各处手写的
  `uppercase tracking-wider`（详情页"专辑/艺术家"、表头、FolderView 分组标题都在重复写）。

### 2.3 间距

- ✅已验证：自定义 `sp1…sp8` 令牌 **零引用**（组件全用 Tailwind 默认 `gap-2/p-4`）。
  Tailwind 默认就是 4px 网格，与 `sp*` 完全等价 → **建议直接从 `tailwind.config.js` 删除 `spacing.sp*`**，消除"两套间距"困惑。

### 2.4 阴影 / 层级（新增刻度）

现状用 `shadow-sm/md/xl` 随手取，缺少语义。新增并在 Tailwind 映射：

```
--shadow-1 (card-hover)   : 0 1px 2px rgba(0,0,0,.06), 0 1px 3px rgba(0,0,0,.05)
--shadow-2 (popover/menu) : 0 4px 12px rgba(0,0,0,.10)
--shadow-3 (dialog)       : 0 12px 32px rgba(0,0,0,.16)
```

用途：专辑卡 hover 微抬（shadow-1）、右键/下拉菜单（shadow-2）、对话框（shadow-3）。

### 2.5 z-index（新增刻度）

现状 ad-hoc `z-10/50/[100]/[101]`。映射语义令牌：`dropdown(50)` / `sticky(40)` / `modal(100)` / `toast(110)`。

### 2.6 动效

- ✅已验证：`duration-120` 出现在 5 个文件共 8 处，**不是 Tailwind 默认刻度**（默认只有 75/100/150/200/300…），
  → 该 class **不生成样式、静默失效**。组件靠 `transition-colors` 自带的默认 150ms 勉强工作。
  修复二选一：①在 theme 里加 `transitionDuration: { 120: '120ms' }`；②统一改 `duration-150`。**建议方案①**（保留原设计意图 120ms）。
- 统一 `--ease` 缓动曲线。

### 2.7 为暗色主题留口子（本次不实现）

把 `:root` 里的令牌原样保留，未来新增暗色只需加 `.dark { --background: ...; ... }` 覆盖块，
组件代码无需改动。本方案的所有颜色都走令牌、不写死十六进制，确保这一点成立。

---

## 三、全局与布局

### 3.1 标题栏 `TitleBar.vue`（h-9）

- **窗口控制按钮 hit area 偏小**：`size="icon-sm"`（h-8=32px）放在 36px 高的栏里，
  上下各留 2px **不可悬停**的缝。建议改为**满高**按钮（`h-9` + `rounded-none`，宽度 ~46px），
  对齐 VS Code / 原生窗口控件的交互预期。
- 最大化态用 `Minimize2` 当"还原"图标略怪；可换成更贴近 restore 的双叠方框（lucide 0.468 无专用 restore，可用 `Square` 叠加或保持现状，纯观感）。
- "Caw" 字标用 `text-caption`(11px) 偏小，建议提到 12–13px 或做一个小 logo lockup。

### 3.2 侧栏 `Sidebar.vue`（w-60）

- **激活指示条未垂直居中（bug）** ✅已验证：
  `<div class="absolute left-0 w-0.5 h-5 bg-primary rounded-r-full" />` 在 `relative` 按钮里，
  **没有 `top`/`translate`**，默认贴在内容区顶部而非按钮中线。
  修复：`top-1/2 -translate-y-1/2`（或 `inset-y-0 my-auto`）。
- **图标左右跳动**：为给指示条让位，图标加了 `ml-0.5`，但该偏移在**非激活态也存在**，
  导致切换激活时图标位置微跳。修复：恒定 `pl-3`，指示条放在 padding 内（激活/非激活内边距一致）。
- 顶部分割线 `v-if` 条件冗长难读（`view.nav==='playlists' || (len>0 && nav!=='playlists')`），建议化简。
- 底部 "{{filteredTracks.length}} tracks" 会随搜索变化，易误解；可改显示曲库总数或带"已筛选/全部"语义。
- （可选）导航项上方加一个 "资料库" overline 分组标签，增强结构感。

### 3.3 播放栏 `PlayerBar.vue`（h-20）

- **主播放按钮与详情页不一致**：此处 `w-8 h-8 bg-foreground`（32px、黑），
  而 `AlbumDetail/ArtistDetail` 头部是 `w-12 h-12 bg-primary`（48px、绿）。
  统一规则（建议）：**主播放 CTA 用 primary 绿**，尺寸 36–40px。两处对齐。
- 80px 高度内塞"传输按钮行 + 进度行"偏挤（`gap-1`）。建议栏高提到 ~88px 或收紧行间距。
- 进度两侧时间标签 `w-8`(32px) 对 `59:59` 这种 5 字符略紧，给到 `w-9`/`w-10` 更稳。
- 顺序/随机压在**单按钮循环 4 态**里，发现性差（P2 可拆成 shuffle + repeat 两个独立按钮）。
- 左 `w-56` 固 + 右 `w-36` 固 + 中 `max-w-2xl mx-auto`：桌面窗口（默认 ≥900px）可接受，
  但极窄窗口会碰撞，建议给中部 `min-w-0` 并在窄窗口隐藏音量标签（低优先）。

### 3.4 内容区 `Content.vue`

- 搜索栏输入框去掉了边框/背景/聚焦环（`border-none bg-transparent focus-visible:ring-0`），
  导致**无聚焦反馈**。建议保留极淡的 focus 下划线或聚焦时图标变色。

---

## 四、内容视图（问题最集中的地方）

### 4.1 曲目表 `TrackTable.vue`

当前播放行 **同时有 5 重指示**，且其中一条会撑动布局：

1. 序号位 `#` 替换为 Play 图标（`fill-primary`）✅ 保留
2. 序号/标题 `text-primary`
3. **多余的 `w-0.5 h-4 bg-primary mr-3` 竖条 —— 会把标题列右推，触发重排**（代码用 `measure()` 打补丁，仍 jank）❌ 删除
4. 整行 `bg-primary/10` 底色 ✅ 保留
5. 标题 `text-body-md` 加粗 ✅ 保留

→ 建议：**保留 1 + 4 + 5，删除 3**，2 由 5 的加粗覆盖。一处指示规范，全表统一。

其余问题：

- **"+" 按钮 hover 永不触发（bug）** ✅已验证：按钮用 `group-hover:opacity-100`，但
  **所在行 div 没有 `group` 类**，所以一直停在 `opacity-40`。修复：给行加 `group`。
- **列宽固定 px、不可伸缩**：index 40 + title 400 + artist 240 + album 240 + duration 80 = 1000px，
  末列 `flex-shrink-0`。宽窗标题列后大片留白；窄窗溢出。修复：**标题列改 `flex-1 min-w-0`**，其余固定。
- **行分割线与表头不一致**：表头 `border-border`(100%)，行 `border-border/40`(40%) 偏淡，表格显"散"。
  建议统一为 `border-border/50`，或干脆启用斑马纹（见 2.1）替代细线。
- **下拉/右键菜单**无 `max-h`/滚动/方向键导航，末行处可能溢出视口。已有位置钳制，补 `max-h-64 overflow-auto`。
- 序号单元格已有 Play 图标，额外竖条冗余（见上）。

### 4.2 专辑网格 `AlbumGrid.vue`

- 卡片 hover **只变背景**，无抬升、无悬浮播放按钮。建议：hover 加 `shadow-1` + `scale-[1.02]`，
  封面中央浮出一个圆形播放按钮（参照 Apple/Spotify）。
- 无封面卡片**全是重复灰块**（`CoverArt` 的 `bg-elevated-hover` + Music 图标）。建议见 4.3 渐变占位。

### 4.3 封面 `CoverArt.vue`

- **无加载骨架**：拉取封面字节前是空盒，会闪。建议 `coverUrl` 未就绪时显示 `animate-pulse` 骨架。
- **无封面占位重复**：建议按标题/专辑名 hash 出一个**确定性渐变**（hue 由 hash 决定），
  让无封面的卡片也有视觉差异，而不是一片灰。

### 4.4 详情页头部 `AlbumDetail.vue` / `ArtistDetail.vue`

- 大播放按钮 `bg-primary` —— 与播放栏统一（见 3.3）。
- **返回导航不一致**：`AlbumDetail/ArtistDetail` 用文字"返回"按钮，`PlaylistDetail` 用 `ArrowLeft` 图标。
  统一为 **`ArrowLeft` 图标按钮（可带"返回"文字）**。
- `ArtistDetail` 头像恒为灰圆 + Users 图标，重复感强。改用 **首字母头像 / 渐变占位**（同 4.3 策略）。
- （P2）头部背景纯平；将来可做"由封面主色衍生的淡彩/虚化背景"（需取色，本次不做）。

### 4.5 艺术家列表 `ArtistList.vue`

- 头像同上，灰圆 + Users 重复 → 首字母/渐变头像。
- 此处用裸 `text-xs` 而非 `text-caption` 令牌，统一到令牌。

### 4.6 文件夹视图 `FolderView.vue`

- **当前播放用 `♫` 字符**，与 TrackTable(Play 图标)、PlaylistDetail(Play 图标) 不一致。
  统一为 Play 图标规范（见五）。
- 面包屑只剩"当前文件夹名 + 返回"，深层目录丢上下文。建议改为**完整路径可点分段面包屑**。
- 列表行高/内边距与 TrackTable 不同（可接受，但注意观感连贯）。

### 4.7 播放列表详情 `PlaylistDetail.vue`

- 删除按钮用 `text-red-400 hover:text-red-300` —— 这是**暗色调的红，在亮底下对比不足** ✅已验证。
  改为 `text-red-600 hover:text-red-700`（或统一走新 `--destructive` 令牌）。
- 内联的"删除确认框"**重复造了 `PlaylistDialog` 的遮罩/面板标记**。抽取一个共享 `ConfirmDialog` 原语。
- `GripVertical` 拖拽手柄是**占位、未实现**。要么实现拖拽排序，要么移除以免误导。
- 当前播放指示（Play 图标 + 加粗 + primary + `bg-primary/10`）3 重 —— 比表格克制，可作"规范基准"。

### 4.8 设置 `Settings.vue`

- 自定义开关无 a11y（无 `role/aria`、不可键盘操作）。建议用 `radix-vue` 的 `Switch`（项目已装 radix）。
- 开关关闭态旋钮 `bg-foreground` 在 `bg-border` 上对比偏低，微调。
- 卡片 `bg-elevated border` 结构清晰，OK。

### 4.9 对话框 `PlaylistDialog.vue` + 删除确认

- **无进/出场动画**，瞬时出现。加 Vue `<Transition>` 做 fade + scale。
- 确认按钮配色不统一（PlaylistDialog `bg-primary`，删除框 `bg-red-500`）—— 走 `--destructive` 令牌后自然统一。
- `Esc` 关闭只在 `PlaylistDialog` 的 input 上生效，遮罩层未处理全局 Esc。

---

## 五、一致性规范（跨视图统一）

定义以下"唯一规范"，所有列表/视图遵守：

| 项 | 规范 |
|---|---|
| **当前播放指示** | 序号位 → Play 实心图标(primary)；当前行 → 极淡底色 `primary/10`；标题 → semibold。**删除**竖条、删除 `♫` 字符 |
| **返回导航** | 统一 `ArrowLeft` 图标按钮（+ 可选"返回"文字） |
| **破坏性操作颜色** | 统一 `--destructive` 令牌（亮底用 red-600 级，非 red-300/400） |
| **聚焦环** | 统一 `--ring`，Button/Input/Slider 一致 |
| **图标尺寸** | 小 3.5 / 默认 4 / 大 5，三档封顶，不再随手 3/3.5/4 混用 |
| **overline 小标签** | 统一一个 `.text-overline` 工具类替代各处手写 `uppercase tracking-wider` |
| **无封面占位** | 统一"hash 渐变 + 骨架加载"策略，全项目复用 `CoverArt` |

---

## 六、可访问性（顺带修）

- `--faint-foreground`(2.8:1) 仅用于装饰文字，信息性文字不得低于 `--muted-foreground`。
- 颜色态（当前播放）均配图标，不只靠颜色 —— 已达标，保持。
- 对话框 `Esc` 全局关闭、Switch 键盘可达、菜单方向键导航（当前均缺）。
- 所有交互元素保留 `focus-visible` 环（统一颜色）。

---

## 七、优先级与实施顺序

> P0 = 真 bug / jank，影响体验，风险低；P1 = 一致性与视觉质感；P2 = 锦上添花。

### P0 ✅ 已完成（已构建验证 `npm run build` 通过）

| # | 项 | 文件 | 类型 |
|---|---|---|---|
| 1 | 侧栏激活指示条垂直居中 + 消除图标跳动 | `Sidebar.vue` | bug |
| 2 | 表格行补 `group` 类，修复"+"按钮 hover 不触发 | `TrackTable.vue` | bug |
| 3 | 删除表格当前播放的多余竖条（消除重排 jank） | `TrackTable.vue` | jank |
| 4 | 表格标题列改 `flex-1`，修复宽/窄窗布局 | `TrackTable.vue` | 布局 |
| 5 | 窗口控制按钮满高 hit area | `TitleBar.vue` | bug |
| 6 | `duration-120` 非法值 → theme 注册 120ms | `tailwind.config.js` + 各组件 | bug |
| 7 | 播放列表删除按钮 red-300/400 → red-600/700 | `PlaylistDetail.vue` | 配色 bug |

### P1 ✅ 已完成（已构建验证 `npm run build` 通过）

8. 设计令牌：新增 `--destructive` / `--ring` / shadow / z-index / overline 工具类，全局替换裸值
9. 统一"当前播放指示"规范（FolderView 去 `♫`、各视图对齐）
10. 统一返回导航为 `ArrowLeft`
11. 启用或删除斑马纹令牌（二选一）；统一表格行分割线
12. `CoverArt` 加载骨架 + 无封面渐变占位
13. 专辑卡 hover 抬升 + 悬浮播放按钮
14. 专辑/艺术家头像改首字母/渐变占位（去重复灰块）
15. 播放栏主按钮与详情页统一（primary 绿 + 尺寸）
16. 对话框进出场动画 + 抽取共享 `ConfirmDialog`
17. 清理死令牌（`sp1…sp8`、未用字体资源）；决定 MapleMono 去留

### P2 ✅ 已完成（已构建验证 `npm run build` 通过）

18. 搜索栏聚焦反馈
19. 播放栏 shuffle / repeat 拆成两个独立按钮
20. FolderView 完整路径面包屑
21. 侧栏分组 overline 标签
22. 播放列表拖拽排序（或移除手柄）
23. 设置 Switch 改 radix（a11y）
24. 详情页"封面主色虚化背景"（需取色，较重）

---

## 八、不在本次范围

- **暗色主题**：本次决策保留亮色。令牌已结构化，将来加 `.dark {}` 覆盖即可，无需改组件。
- 功能性新增（如歌词、均衡器、拖拽落盘导入等）。

---

## 九、验证方式

每批改完后：

```bash
npm run build      # vue-tsc 类型检查 + vite 构建，确认无类型/编译错误
cargo tauri dev    # 实际窗口逐项目检（重点：侧栏激活、表格当前播放/hover、播放栏、对话框动画）
```

无自动化 UI 测试（与项目现状一致）；以 `cargo tauri dev` 人工目检为准。
