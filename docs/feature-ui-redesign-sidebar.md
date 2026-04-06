# UI 重设计：侧边栏 + 多页面布局

## 状态：已完成 ✅

## 概述

将 Input0 设置窗口从单页滚动表单重构为侧边栏 + 内容区的多页面布局，参考 Typeless 应用的设计风格。

## 需求来源

用户参考 Typeless 应用界面，要求：
- 左侧导航栏 + 右侧内容区布局
- 保留实际功能（排除：登录注册、Pro 计划、邀请返佣、反馈）
- 历史记录仅保留上一次语音输入和对应优化内容
- 参考设计风格和布局，不硬加图片中没有的功能

## 技术方案

### 导航方案

使用 `useState<PageId>` 状态管理页面切换（非 react-router），因为 `/overlay` 路由必须保持独立。

```typescript
type PageId = "home" | "history" | "settings";
```

### 历史记录持久化

新增 `history-store.ts`（Zustand + localStorage），独立于 `recording-store`（每次 idle/cancelled 重置）。

### 后端改动

`PipelineState::Done` 增加 `transcribed_text` 字段，使前端能同时获取原始转录文本和优化后文本。

## 文件变更清单

### 新增文件

| 文件 | 说明 |
|------|------|
| `src/stores/history-store.ts` | Zustand store，localStorage 持久化上一次转录结果 |
| `src/components/Sidebar.tsx` | 侧边栏组件：Logo、导航（首页/历史记录/设置）、版本号 |
| `src/components/SettingsPage.tsx` | 从 Settings.tsx 抽取的设置内容组件 |
| `src/components/HomePage.tsx` | 首页：欢迎信息、模型状态卡片、快捷键/语言信息卡片、使用说明 |
| `src/components/HistoryPage.tsx` | 历史记录页：上次转录/优化结果展示、复制按钮、空状态 |

### 修改文件

| 文件 | 说明 |
|------|------|
| `src/pages/Settings.tsx` | 重写为布局外壳：Sidebar + 内容区 + AnimatePresence 页面切换 |
| `src/hooks/useTauriEvents.ts` | done 事件时调用 history-store 保存结果 |
| `src/index.css` | 新增侧边栏主题变量（light + dark） |
| `src-tauri/tauri.conf.json` | 窗口尺寸 800×600 → 960×640 |
| `src-tauri/src/pipeline.rs` | `Done` 变体增加 `transcribed_text` 字段 |

## 设计规范

- 侧边栏宽度：200px 固定
- 页面切换动画：`AnimatePresence mode="wait"`，opacity + y 位移
- CSS 变量前缀：`--theme-sidebar-*`
- 拖拽区域：侧边栏 header 设置 `data-tauri-drag-region`

## 验证

- [x] `tsc` 类型检查通过
- [x] `vite build` 构建成功
- [x] `cargo check` Rust 编译通过
- [x] Overlay 窗口路由未受影响
