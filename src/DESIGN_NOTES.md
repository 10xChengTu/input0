# React 前端 — Design Notes

本文件记录前端关键设计决策，帮助 agent 和开发者理解「为什么这样设计」。

## 双窗口架构

**决策**: Settings 和 Overlay 是两个独立的 Tauri WebView 窗口，通过 BrowserRouter 路由区分 (`/` vs `/overlay`)。

**原因**: 
- Settings 窗口需要标准的 macOS 窗口行为（标题栏、resize、关闭 → 隐藏）
- Overlay 窗口需要透明背景、无边框、always-on-top、忽略鼠标事件
- 两者的窗口属性完全不同，无法共用一个 WebView

**约束**: 两个窗口共享同一个 React 进程，store 状态是共享的。但实际上只有 `recording-store` 需要跨窗口同步（通过 Tauri events 驱动）。

## Zustand 一 Store 一 Domain

**决策**: 4 个独立 store，不合并。

**原因**:
- `recording-store` — 生命周期最短（每次录音），更新频率最高
- `settings-store` — 持久化到后端 config.toml，低频更新
- `history-store` — 追加写，偶尔读
- `theme-store` — 几乎不变

合并到一个 store 会导致不相关的 re-render。独立 store 让每个 domain 的订阅者互不影响。

## 事件驱动 UI 更新

**决策**: 使用 `useTauriEvents` hook 监听后端 `pipeline-state` 事件，驱动 `recording-store` 更新。

**原因**: Pipeline 流程完全由后端控制（快捷键触发），前端不主动轮询。事件驱动是最自然的模式 — 后端状态变化时推送给前端，前端被动更新 UI。

**注意**: `useTauriEvents` 在 Overlay 和 Settings 窗口中都会调用。两个窗口接收相同的事件。

## Sidebar 多页面布局

**决策**: Settings 窗口内用 Sidebar + 条件渲染（状态切换），不用嵌套路由。

**原因**: 页面数量少且固定（3 个：首页/历史/设置），不需要 URL 路由来管理。状态切换更简单，避免了 react-router 嵌套路由的复杂性。

## 动画选择

**决策**: 使用 Framer Motion 而非 CSS 动画。

**原因**: 
- Overlay 的波形动画需要精细的 spring physics 控制
- 页面切换动画需要 `AnimatePresence` 的 exit 动画支持
- Tailwind CSS 的内置动画不够灵活

**约束**: Framer Motion v12+，使用 `motion` 组件而非 `m` 简写（项目统一约定）。

## 液态玻璃 Overlay

**设计**: Overlay 窗口使用 `backdrop-filter: blur()` + 半透明背景，模拟 macOS 液态玻璃效果。

**约束**: 
- 需要 Tauri 窗口配置中开启 `transparent: true`
- macOS 需要 `macos-private-api` feature（已在 Cargo.toml 中启用）
- 窗口背景色通过 Cocoa API 设置（在 `lib.rs` setup 中）
