# Feature: STT 模型切换 + 按需下载 + 语言最佳模型推荐

## 状态：已完成 ✅

## 需求概述

1. **模型切换功能**：支持在 9 个 STT 模型之间切换（Whisper 系列 6 个 + SenseVoice-Small + Paraformer 中文 + Moonshine Base EN）
2. **按需下载**：模型不再内置到应用包，用户需要时手动触发下载
3. **四推理后端**：whisper-rs（Whisper 系列）+ sherpa-onnx（SenseVoice / Paraformer / Moonshine）
4. **语言最佳配置**：每种语言有对应的最佳 STT 模型推荐
5. **切换提示**：用户切换语言时，如果当前模型非最佳配置，提示切换

## 技术方案

### 架构设计

```
AppConfig (stt_model 字段)
    ↓
ModelRegistry (静态注册表: 模型元数据 + 下载URL + 语言推荐)
    ↓
ModelManager (下载/存储/校验)
    ↓
TranscriberBackend trait (统一接口)
    ├── WhisperBackend (whisper-rs)
    ├── SenseVoiceBackend (sherpa-onnx)
    ├── ParaformerBackend (sherpa-onnx)
    └── MoonshineBackend (sherpa-onnx)
    ↓
Pipeline (使用当前激活的 backend)
```

### 支持的模型

| 模型 ID | 显示名称 | 后端 | 大小 | 最佳语言 |
|---------|----------|------|------|---------|
| `whisper-base` | Whisper Base | whisper-rs | ~142MB | 通用（基础） |
| `whisper-small` | Whisper Small | whisper-rs | ~466MB | 通用（性价比高） |
| `whisper-medium` | Whisper Medium | whisper-rs | ~1.4GB | 通用（多语言优秀） |
| `whisper-large-v3` | Whisper Large v3 | whisper-rs | ~2.9GB | auto, fr, de, es, ru |
| `whisper-large-v3-turbo` | Whisper Large v3 Turbo | whisper-rs | ~1.5GB | en |
| `whisper-large-v3-turbo-q5` | Whisper Large v3 Turbo (Q5) | whisper-rs | ~547MB | 通用（量化） |
| `sensevoice-small` | SenseVoice Small | sherpa-onnx | ~228MB | zh, ja, ko |
| `paraformer-zh` | Paraformer 中文 | sherpa-onnx | ~217MB | zh |
| `moonshine-base-en` | Moonshine Base (EN) | sherpa-onnx | ~274MB | en |

### 语言最佳模型映射

| 语言 | 最佳模型 | 原因 |
|------|---------|------|
| zh | sensevoice-small | 中文 CER 显著优于 Whisper |
| en | whisper-large-v3-turbo | 英文 WER 最佳 |
| ja | sensevoice-small | 日语支持良好 |
| ko | sensevoice-small | 韩语支持良好 |
| auto | whisper-large-v3 | 多语言自动检测，Large v3 覆盖语种最广 |
| fr/de/es/ru | whisper-large-v3 | 欧系语言 Whisper Large v3 更优 |

### 模型存储路径

```
~/Library/Application Support/com.input0.dev/models/
  ├── whisper-base/
  │   └── ggml-base.bin
  ├── whisper-small/
  │   └── ggml-small.bin
  ├── whisper-medium/
  │   └── ggml-medium.bin
  ├── whisper-large-v3/
  │   └── ggml-large-v3.bin
  ├── whisper-large-v3-turbo/
  │   └── ggml-large-v3-turbo.bin
  ├── whisper-large-v3-turbo-q5/
  │   └── ggml-large-v3-turbo-q5_0.bin
  ├── sensevoice-small/
  │   ├── model.int8.onnx
  │   └── tokens.txt
  ├── paraformer-zh/
  │   ├── model.int8.onnx
  │   └── tokens.txt
  └── moonshine-base-en/
      ├── preprocess.onnx
      ├── encode.int8.onnx
      ├── uncached_decode.int8.onnx
      ├── cached_decode.int8.onnx
      └── tokens.txt
```

### 关键技术决策

1. **WhisperContext 不再用 OnceLock 全局单例** → 改用 `Arc<Mutex<Option<Box<dyn TranscriberBackend>>>>` 作为 Tauri managed state
2. **模型切换时重新加载** → 旧 context drop，新 context 创建
3. **下载进度通过 Tauri events 通知前端** → `model-download-progress` event
4. **SenseVoice 通过 sherpa-onnx 集成** → sherpa-onnx crate (v1.12) 的 Rust bindings

## 实现状态

- [x] 配置层：添加 stt_model 字段
- [x] 模型注册表模块
- [x] 模型下载管理模块
- [x] TranscriberBackend trait + Whisper 实现
- [x] SenseVoice 后端实现 (sherpa-onnx)
- [x] Paraformer 后端实现 (sherpa-onnx)
- [x] Moonshine 后端实现 (sherpa-onnx)
- [x] Tauri commands
- [x] Pipeline 适配
- [x] 前端 UI：模型选择、下载、语言推荐提示
- [x] 测试
