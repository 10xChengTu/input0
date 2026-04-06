# UI 重设计：暗黑主题 + 动画 + 液态玻璃

## 状态：已完成 ✅

## 需求概述

对 Settings 页面和语音输入指示条（Overlay）进行全面 UI 重设计，提升用户友好性和交互体验。

### 用户需求
1. Settings 页面设计优化
2. 语音输入指示条设计优化
3. 暗黑主题（黑色为主，仅错误信息使用红色）
4. 动画效果（Framer Motion）
5. 响应式布局
6. 指示条参考 Mac 液态玻璃风格
7. 避免页面抖动，提前预留占位
8. 通知信息统一在右上角，黑白色
9. Bug 修复：切换模型后 Status 区域显示实际活动模型名称（非硬编码 "Whisper Model"）
10. Bug 修复：切换模型后推荐 banner 自动消失

## 技术方案

### 技术选型
- **动画库**：Framer Motion v12（React 19 兼容）
- **样式方案**：纯 Tailwind CSS v4（`@theme {}` 自定义属性），不引入组件库
- **弃选 Shadcn**：Shadcn 目前不兼容 TW v4 `@theme` 语法

### 设计系统（`src/index.css @theme`）

| Token | 值 | 用途 |
|-------|---|------|
| `--color-bg-primary` | `#000000` | 页面背景 |
| `--color-bg-card` | `rgba(255,255,255,0.04)` | 卡片背景 |
| `--color-border-default` | `rgba(255,255,255,0.08)` | 默认边框 |
| `--color-text-primary` | `#f5f5f5` | 主文字 |
| `--color-text-secondary` | `#a3a3a3` | 次级文字 |
| `--color-accent` | `#e5e5e5` | 强调色（单色按钮） |
| `--color-error` | `#ef4444` | 仅用于错误 |
| `--color-glass-bg` | `rgba(0,0,0,0.55)` | 液态玻璃背景 |

### 动画规范
- **Quick spring**：`stiffness:400, damping:30, mass:0.5`（按钮、toast）
- **Natural spring**：`stiffness:260, damping:25, mass:0.8`（区域入场）
- **防抖动策略**：`AnimatePresence` + `height: auto` 动画避免布局跳变

## 修改文件清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/index.css` | 重写 | TW v4 `@theme` 暗黑设计系统 |
| `src/components/Toast.tsx` | 重写 | 暗黑主题 + Framer Motion 弹簧动画 |
| `src/components/WaveformAnimation.tsx` | 重写 | CSS 动画 → Framer Motion 交错条形动画 |
| `src/components/ProcessingIndicator.tsx` | 重写 | SVG spinner → 3 个弹跳圆点 |
| `src/pages/Settings.tsx` | 重写 | 全暗黑主题、响应式、Framer Motion 区域入场动画 |
| `src/pages/Overlay.tsx` | 重写 | 液态玻璃效果、AnimatePresence 状态切换 |
| `src/stores/settings-store.ts` | 修改 | `switchModel()` 增加 `checkModelRecommendation()` 调用 |
| `package.json` | 修改 | 新增 `framer-motion` 依赖 |

## Bug 修复

### 1. Status 区域硬编码 "Whisper Model"
**问题**：切换到非 Whisper 模型后，Status 仍显示 "Whisper Model"
**修复**：`activeModel?.display_name || "Speech Model"` 动态显示活动模型名

### 2. 模型推荐 Banner 不消失
**问题**：切换模型后推荐 banner 仍然显示
**修复**：在 `switchModel()` 末尾调用 `checkModelRecommendation(get().language)`

## 验证结果

- ✅ `npx tsc --noEmit` — 零错误
- ✅ `pnpm build` — 构建成功（299ms）
- ✅ 无旧浅色主题残留（`bg-gray-50`, `ring-indigo-` 等均已清除）
- ✅ 无硬编码 "Whisper Model"
- ✅ `AnimatePresence` 在 Settings.tsx 和 Overlay.tsx 中均已使用
- ✅ `backdrop-blur-[40px]` 液态玻璃效果在 Overlay.tsx 中已实现
