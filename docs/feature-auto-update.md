# Feature: 应用内自动更新

## 需求

桌面端应用需要在应用内通知用户版本更新，并支持一键下载安装。更新通知应非侵入式（不在 Overlay 浮层弹出，避免打断语音输入体验）。

## 技术方案

### 插件选型

使用 Tauri v2 官方插件 `tauri-plugin-updater` + `tauri-plugin-process`：

- **tauri-plugin-updater**: 检查更新、下载安装更新包
- **tauri-plugin-process**: 安装完成后重启应用（`relaunch()`）

### 更新源

GitHub Releases（`https://github.com/10xChengTu/input0`）：
- Endpoint: `https://github.com/10xChengTu/input0/releases/latest/download/latest.json`
- Tauri 构建时自动生成 `latest.json` 签名文件

### 签名机制

Tauri Updater 要求对更新包进行签名验证：
- `tauri.conf.json` 中配置 `plugins.updater.pubkey`（当前为占位符）
- 构建时需设置环境变量 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- CI/CD 中通过 `tauri signer generate -w ~/.tauri/myapp.key` 生成密钥对

## 实现细节

### 后端（Rust）

在 `lib.rs` 中注册两个插件：

```rust
.plugin(tauri_plugin_updater::Builder::new().build())
.plugin(tauri_plugin_process::init())
```

### 前端状态管理

新建 `src/stores/update-store.ts`（Zustand store）：

| 状态 | 类型 | 说明 |
|------|------|------|
| `updateAvailable` | `boolean` | 是否有可用更新 |
| `updateVersion` | `string \| null` | 新版本号 |
| `updateBody` | `string \| null` | 更新说明（Release Notes） |
| `isChecking` | `boolean` | 正在检查更新中 |
| `isDownloading` | `boolean` | 正在下载更新中 |
| `downloadProgress` | `number` | 下载进度 (0-100) |
| `error` | `string \| null` | 错误信息 |

核心方法：
- `checkForUpdates()`: 调用 `check()` API 检查更新，缓存 Update 对象
- `downloadAndInstall()`: 调用 `update.downloadAndInstall(onEvent)` 下载并安装，完成后 `relaunch()`
- `dismissUpdate()`: 关闭更新通知

### UI 集成

#### Sidebar 更新徽标

- 底部版本号区域通过 `getVersion()` API 动态显示当前版本
- 有可用更新时，版本号旁显示 ping 动画红点（与 STT 模型状态指示器同风格）

#### SettingsPage 关于与更新 Section

在 general tab 末尾（user tags section 之后）新增"关于与更新"区域：

1. **当前版本号** + **检查更新按钮**（带 loading spinner）
2. **更新可用时**：显示新版本号、Release Notes、下载安装按钮 + 忽略按钮
3. **下载中**：进度条 + 百分比（复用项目已有的进度条样式）
4. **错误状态**：显示错误信息（复用项目已有的 error alert 样式）

### i18n

在 `Translations` 接口中新增 `update` 类别，包含：
- `title` / `currentVersion` / `checkForUpdates` / `checking`
- `availableMessage(version)` / `downloadAndInstall` / `dismiss`
- `upToDate` / `downloadComplete`

### Capabilities 权限

`src-tauri/capabilities/default.json` 中添加：
- `"updater:default"` — 允许检查和下载更新
- `"process:allow-restart"` — 允许重启应用

### Tauri 配置

`tauri.conf.json` 新增：

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "YOUR_PUBLIC_KEY_HERE",
      "endpoints": [
        "https://github.com/10xChengTu/input0/releases/latest/download/latest.json"
      ]
    }
  }
}
```

## 发布流程（CI/CD 配置）

> 以下步骤需要在 GitHub Actions 或本地手动执行，不在应用代码中实现。

1. 生成签名密钥对：`tauri signer generate -w ~/.tauri/myapp.key`
2. 将公钥填入 `tauri.conf.json` 的 `plugins.updater.pubkey`
3. 在 CI 中设置 secrets：`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
4. 构建命令：`pnpm tauri build --bundles app`（自动生成 `.tar.gz` + `.tar.gz.sig` 签名文件）
5. 将构建产物和 `latest.json` 上传至 GitHub Release

## 文件清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src-tauri/Cargo.toml` | 修改 | 添加 tauri-plugin-updater + tauri-plugin-process 依赖 |
| `src-tauri/tauri.conf.json` | 修改 | createUpdaterArtifacts + plugins.updater 配置 |
| `src-tauri/capabilities/default.json` | 修改 | updater:default + process:allow-restart 权限 |
| `src-tauri/src/lib.rs` | 修改 | 注册两个新插件 |
| `src/stores/update-store.ts` | 新建 | Zustand update store |
| `src/i18n/types.ts` | 修改 | 添加 update 翻译类型 |
| `src/i18n/zh.ts` | 修改 | 中文 update 翻译 |
| `src/i18n/en.ts` | 修改 | 英文 update 翻译 |
| `src/components/Sidebar.tsx` | 修改 | 动态版本号 + 更新徽标 |
| `src/components/SettingsPage.tsx` | 修改 | 关于与更新 section |
| `package.json` | 修改 | 安装前端依赖 |

## 实现状态

- [x] 后端插件注册
- [x] Tauri 配置（占位符 pubkey + endpoint）
- [x] Capabilities 权限
- [x] 前端 update store
- [x] i18n 翻译
- [x] Sidebar 更新徽标
- [x] SettingsPage 关于与更新 section
- [x] TypeScript 类型检查通过
- [ ] 签名密钥配置（需用户手动设置）
- [ ] CI/CD 自动发布流程（需用户单独配置）
