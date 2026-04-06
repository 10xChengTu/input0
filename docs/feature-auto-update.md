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

## CI/CD 自动发布

### GitHub Actions Workflow

`.github/workflows/release.yml` — 推送到 `release` 分支时自动触发。

**构建平台**：`macos-latest`（Apple Silicon），Intel Mac 用户可通过 Rosetta 2 运行。

**流程**：
1. `changelog` job（ubuntu）：`git-cliff` 解析 Conventional Commits，生成分类 Release Notes
2. `release` job（macOS ARM64）：安装 Rust + Node.js 22 + pnpm + cmake
3. `tauri-apps/tauri-action@v0` 自动构建、签名、创建 GitHub Release（使用生成的 changelog 作为 Release Notes）
4. 产物自动上传：`.dmg`、`.app.tar.gz`、`.app.tar.gz.sig`、`latest.json`
5. `latest.json` 供 updater 插件检查更新

**Release 命名**：`v{version}`（version 自动读取自 `tauri.conf.json`）

### 前置配置（一次性）

#### 1. 签名密钥

```bash
# 生成密钥对（已完成）
pnpm tauri signer generate -w ~/.tauri/myapp.key
```

- 公钥已填入 `tauri.conf.json` 的 `plugins.updater.pubkey`
- 私钥文件保存在 `~/.tauri/myapp.key`

#### 2. GitHub Secrets

在 GitHub 仓库 Settings → Secrets and variables → Actions 中添加：

| Secret 名称 | 值 | 说明 |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | `~/.tauri/myapp.key` 文件完整内容 | Updater 签名私钥 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成密钥时设置的密码 | 私钥密码（无密码则留空） |

> `GITHUB_TOKEN` 由 Actions 自动提供，无需手动配置。

### Conventional Commits 规范

Commit message 需遵循 [Conventional Commits](https://www.conventionalcommits.org/) 格式，Release Notes 会自动按类型分类：

| 前缀 | Release Notes 分类 | 示例 |
|------|-------------------|------|
| `feat:` | Features | `feat: add noise reduction` |
| `fix:` | Bug Fixes | `fix: audio capture crash on M3` |
| `improvement:` | Feature Improvements | `improvement: faster model loading` |
| `docs:` | Docs | `docs: update README` |
| `style:` | Styling | `style: fix sidebar alignment` |
| `refactor:` | Code Refactoring | `refactor: extract audio pipeline` |
| `perf:` | Performance Improvements | `perf: reduce memory usage` |
| `test:` | Tests | `test: add converter unit tests` |
| `build:` | Build System | `build: upgrade tauri to v2.1` |
| `ci:` | CI | `ci: add x86_64 build target` |
| `revert:` | Reverts | `revert: undo hotkey change` |
| `types:` | Types | `types: fix AppError variants` |
| `chore:` | Chores | `chore: update dependencies` |

不符合 Conventional Commits 格式的 commit 不会出现在 Release Notes 中。配置文件：`cliff.toml`。

### 发布流程

```
master (开发) ──push/merge──▸ release (发布) ──auto──▸ GitHub Actions ──▸ GitHub Release
```

1. 在 `master` 分支完成开发、测试
2. 更新 `tauri.conf.json` 中的 `version` 字段（如 `0.1.0` → `0.2.0`）
3. 将 `master` 合并/推送到 `release` 分支
4. GitHub Actions 自动触发构建
5. 构建成功后自动创建 GitHub Release（tag: `v0.2.0`），上传所有产物
6. 已安装用户的 updater 插件检测到 `latest.json` 更新，提示升级

### 发布产物

每次 Release 包含以下资产：

| 文件 | 用途 |
|------|------|
| `Input0_x.y.z_aarch64.dmg` | Apple Silicon Mac 安装包 |
| `Input0.app.tar.gz` | Updater 更新包 |
| `Input0.app.tar.gz.sig` | 更新包签名文件 |
| `latest.json` | Updater 版本检查 endpoint |

> Intel Mac 用户可通过 macOS 内置的 Rosetta 2 运行 ARM 版本。

### 注意事项

- 每次发布前**必须**更新 `tauri.conf.json` 的 `version`，否则 Release tag 冲突会导致构建失败
- `concurrency: release`：重复推送会取消进行中的构建，只保留最新的
- 仅构建 Apple Silicon（aarch64），Intel Mac 通过 Rosetta 2 兼容运行

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
| `.github/workflows/release.yml` | 新建 | CI/CD 自动发布 workflow |
| `cliff.toml` | 新建 | git-cliff Conventional Commits 分类配置 |

## 实现状态

- [x] 后端插件注册
- [x] Tauri 配置（占位符 pubkey + endpoint）
- [x] Capabilities 权限
- [x] 前端 update store
- [x] i18n 翻译
- [x] Sidebar 更新徽标
- [x] SettingsPage 关于与更新 section
- [x] TypeScript 类型检查通过
- [x] 签名密钥配置（pubkey 已填入 tauri.conf.json）
- [x] CI/CD 自动发布流程（.github/workflows/release.yml）
- [ ] GitHub Secrets 配置（TAURI_SIGNING_PRIVATE_KEY + PASSWORD）
