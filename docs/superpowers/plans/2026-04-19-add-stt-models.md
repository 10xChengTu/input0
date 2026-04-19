# 新增三款 STT 模型 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 向 Input0 模型注册表新增三款 STT 模型：FireRedASR v1（中文 SOTA，1.74 GB）、Paraformer 三语版（中/英/粤，245 MB）、Zipformer 中文 CTC（367 MB）。

**Architecture:** 复用现有 `TranscriberBackend` trait + `OfflineRecognizer` 架构。新增 2 个 backend 文件（FireRedASR、ZipformerCTC），1 个 backend 零新增（Paraformer 三语版复用 `ParaformerBackend`）。`BackendKind` 枚举扩展 2 变体。前端零改动（UI 由 registry 驱动）。

**Tech Stack:** Rust + `sherpa-onnx = "1.12"` crate（已集成）+ Tauri v2。测试走现有 `src-tauri/src/models/tests.rs` 模式——只测 registry + manager，不测 backend 实例化（需真实模型文件，现有 4 个 backend 都无单元测试）。

**Design Decision Log:**

- **Zipformer `bbpe.model` 不下载**：经核查 `sherpa-onnx-1.12.34` crate 源码 `offline_asr.rs:255`，`OfflineZipformerCtcModelConfig` 只有一个 `model: Option<String>` 字段，不暴露 `bpe_vocab`。因此 `bbpe.model` 无处可传，不下载更干净。实现与此 plan 一致后再回头把 feature doc 中"下载但不使用"的措辞更正为"不下载"（见 Task 7）。
- **`fire-red-asr-v1` 不进语言推荐池**：体积 1.74 GB 太大，作为手动选择项。中文推荐继续用 SenseVoice + Paraformer-zh。
- **`paraformer-trilingual` 只推粤语 `yue`**：粤语当前无推荐，此款填补空白；中文/英文仍由现有推荐项占位。
- **`zipformer-ctc-zh` 不进推荐池**：作为尝鲜备选。

---

## File Structure

**Create (3 files):**
- `src-tauri/src/stt/fire_red_asr_backend.rs` — FireRedASR AED backend 封装 `OfflineFireRedAsrModelConfig`（encoder + decoder + tokens）
- `src-tauri/src/stt/zipformer_ctc_backend.rs` — Zipformer CTC backend 封装 `OfflineZipformerCtcModelConfig`（单 model + tokens）
- 无新建文档——`docs/feature-add-stt-models.md` 已存在

**Modify (5 files):**
- `src-tauri/src/models/registry.rs` — `BackendKind` 枚举 +2 变体；追加 3 条 `ModelInfo` + 对应 `ModelFile` 常量
- `src-tauri/src/models/manager.rs` — 新增 `fire_red_asr_model_paths` 和 `zipformer_ctc_model_paths`
- `src-tauri/src/models/tests.rs` — 追加测试覆盖新模型的 `get_model` / `is_downloaded` / 粤语推荐
- `src-tauri/src/stt/mod.rs` — `pub mod fire_red_asr_backend; pub mod zipformer_ctc_backend;`
- `src-tauri/src/lib.rs` — 在 `load_stt_model` 的 `match info.backend` 添加 2 条分支 + 顶部 `use` 声明
- `docs/feature-add-stt-models.md` — Task 7 中更新状态和 `bbpe.model` 决策
- `docs/feature-model-switching.md` — Task 7 中更新模型清单（9 → 12 款）
- `docs/research-local-stt-models.md` — Task 7 中更新方案 C 的模型矩阵
- `CLAUDE.md` — Task 7 中更新 Documentation Map 校验日期

---

## Task 1: 扩展 `BackendKind` 枚举 + `load_stt_model` 占位分支

**Files:**
- Modify: `src-tauri/src/models/registry.rs`
- Modify: `src-tauri/src/lib.rs`

> 为什么一起改：加 enum 变体后 `load_stt_model` 的 `match` 会 non-exhaustive 编译失败。本 task 一次性把 2 个变体和 2 条占位 match 分支全加上，保持 `cargo check` 绿。占位分支在 Task 4、Task 6 会被替换成真实调用。

- [ ] **Step 1: 扩展枚举**

Edit `src-tauri/src/models/registry.rs:7-11`。将：

```rust
pub enum BackendKind {
    Whisper,
    SenseVoice,
    Paraformer,
    Moonshine,
}
```

替换为：

```rust
pub enum BackendKind {
    Whisper,
    SenseVoice,
    Paraformer,
    Moonshine,
    FireRedAsr,
    ZipformerCtc,
}
```

- [ ] **Step 2: 在 `load_stt_model` 加 2 条占位分支**

在 `src-tauri/src/lib.rs` 的 `load_stt_model` 函数中，紧跟在 `BackendKind::Moonshine` 分支闭合 `}` 之后（约第 425 行，`match info.backend` 的大括号结束之前），追加：

```rust
        BackendKind::FireRedAsr => {
            return Err(errors::AppError::Whisper(
                "FireRedAsr backend not yet wired up".to_string(),
            ));
        }
        BackendKind::ZipformerCtc => {
            return Err(errors::AppError::Whisper(
                "ZipformerCtc backend not yet wired up".to_string(),
            ));
        }
```

- [ ] **Step 3: 验证编译 + 测试**

Run: `cd src-tauri && cargo check --lib`
Expected: 编译通过，无新增 error。

Run: `cd src-tauri && cargo test --lib models::tests`
Expected: 现有 11 个 models 测试全部通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models/registry.rs src-tauri/src/lib.rs
git commit -m "feat(stt): 扩展 BackendKind 枚举（FireRedAsr + ZipformerCtc 占位）"
```

---

## Task 2: 注册 `paraformer-trilingual` 模型

**Files:**
- Modify: `src-tauri/src/models/registry.rs`
- Modify: `src-tauri/src/models/tests.rs`

- [ ] **Step 1: 写测试（TDD）**

追加到 `src-tauri/src/models/tests.rs` 末尾：

```rust
#[test]
fn test_paraformer_trilingual_registered() {
    let model = registry::get_model("paraformer-trilingual")
        .expect("paraformer-trilingual must be registered");
    assert_eq!(model.backend, registry::BackendKind::Paraformer);
    assert_eq!(model.files.len(), 2, "should have model + tokens");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"model.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_paraformer_trilingual_recommended_for_cantonese() {
    let recs = registry::recommended_models_for_language("yue");
    let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
    assert!(ids.contains(&"paraformer-trilingual"),
        "paraformer-trilingual should be recommended for Cantonese, got: {:?}", ids);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --lib models::tests::test_paraformer_trilingual`
Expected: FAIL——"paraformer-trilingual must be registered"

- [ ] **Step 3: 追加 `ModelFile` 常量**

在 `src-tauri/src/models/registry.rs` 的 `PARAFORMER_ZH_FILES` 常量块之后（约第 145 行），新增：

```rust
const PARAFORMER_TRILINGUAL_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "model.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-trilingual-zh-cantonese-en/resolve/main/model.int8.onnx",
        size_bytes: 245_000_000,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-trilingual-zh-cantonese-en/resolve/main/tokens.txt",
        size_bytes: 119_000,
        sha1: None,
    },
];
```

- [ ] **Step 4: 追加 `ModelInfo` 到 `ALL_MODELS`**

在 `ALL_MODELS` 中 `paraformer-zh` 条目之后追加：

```rust
    ModelInfo {
        id: "paraformer-trilingual",
        display_name: "Paraformer 中英粤",
        description: "优点：支持中英粤 3 语言，中英代码切换无障碍，阿里出品，推理快\n缺点：体积比 Paraformer 中文版大 15%，纯中文场景精度与其接近，无优势",
        backend: BackendKind::Paraformer,
        total_size_bytes: 245_119_000,
        size_display: "234 MB",
        files: PARAFORMER_TRILINGUAL_FILES,
        best_for_languages: &["yue"],
        recommendation_reason: "粤语识别唯一可用模型，同时支持中英混合",
    },
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cd src-tauri && cargo test --lib models::tests`
Expected: PASS——所有新旧测试通过

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models/registry.rs src-tauri/src/models/tests.rs
git commit -m "feat(stt): 注册 Paraformer 中英粤三语模型（支持粤语）"
```

---

## Task 3: 新建 `FireRedAsrBackend`

**Files:**
- Create: `src-tauri/src/stt/fire_red_asr_backend.rs`
- Modify: `src-tauri/src/stt/mod.rs`

- [ ] **Step 1: 创建 `fire_red_asr_backend.rs`**

Create `src-tauri/src/stt/fire_red_asr_backend.rs`：

```rust
use std::path::Path;

use sherpa_onnx::{OfflineFireRedAsrModelConfig, OfflineRecognizer, OfflineRecognizerConfig};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct FireRedAsrBackend {
    recognizer: OfflineRecognizer,
    model_id: String,
}

unsafe impl Send for FireRedAsrBackend {}
unsafe impl Sync for FireRedAsrBackend {}

impl FireRedAsrBackend {
    pub fn new(
        encoder_path: &Path,
        decoder_path: &Path,
        tokens_path: &Path,
        model_id: &str,
    ) -> Result<Self, AppError> {
        for (label, path) in [
            ("encoder", encoder_path),
            ("decoder", decoder_path),
            ("tokens", tokens_path),
        ] {
            if !path.exists() {
                return Err(AppError::Whisper(format!(
                    "FireRedASR {} file not found: {}",
                    label,
                    path.display()
                )));
            }
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.fire_red_asr = OfflineFireRedAsrModelConfig {
            encoder: Some(encoder_path.to_string_lossy().into_owned()),
            decoder: Some(decoder_path.to_string_lossy().into_owned()),
        };
        config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
        config.model_config.num_threads = 4;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AppError::Whisper("Failed to create FireRedASR recognizer".to_string())
        })?;

        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
        })
    }
}

impl TranscriberBackend for FireRedAsrBackend {
    fn transcribe(&self, audio: &[f32], _language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);

        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or_else(|| AppError::Whisper("Failed to get FireRedASR result".to_string()))?;

        Ok(result.text.trim().to_string())
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::FireRedAsr
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
```

- [ ] **Step 2: 在 `stt/mod.rs` 导出**

Edit `src-tauri/src/stt/mod.rs:1-4`。将：

```rust
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
```

替换为：

```rust
pub mod fire_red_asr_backend;
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
pub mod zipformer_ctc_backend;
```

> 注：`zipformer_ctc_backend` 在 Task 5 才创建，但这里一并声明不会报错——Rust 允许先声明 `pub mod` 再补文件；本 step 只声明 FireRedASR 需要的即可。如果你偏好严格对应关系，本 step 只加 `pub mod fire_red_asr_backend;`，在 Task 5 再加 `pub mod zipformer_ctc_backend;`。

本 step 先**只加 FireRedASR 声明**，避免 Task 3 编译时找不到 zipformer_ctc_backend.rs 而报错：

```rust
pub mod fire_red_asr_backend;
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo check --lib`
Expected: 编译通过，仅有未使用警告（此时 FireRedAsrBackend 还没人用）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/stt/fire_red_asr_backend.rs src-tauri/src/stt/mod.rs
git commit -m "feat(stt): 新增 FireRedASR backend（AED 架构，中文 SOTA）"
```

---

## Task 4: 注册 `fire-red-asr-v1` + 接入 `load_stt_model`

**Files:**
- Modify: `src-tauri/src/models/registry.rs`
- Modify: `src-tauri/src/models/manager.rs`
- Modify: `src-tauri/src/models/tests.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写测试（registry + manager）**

追加到 `src-tauri/src/models/tests.rs` 末尾：

```rust
#[test]
fn test_fire_red_asr_v1_registered() {
    let model = registry::get_model("fire-red-asr-v1")
        .expect("fire-red-asr-v1 must be registered");
    assert_eq!(model.backend, registry::BackendKind::FireRedAsr);
    assert_eq!(model.files.len(), 3, "should have encoder + decoder + tokens");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"encoder.int8.onnx"));
    assert!(paths.contains(&"decoder.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_fire_red_asr_v1_not_in_recommendation_pool() {
    for lang in ["zh", "en", "ja", "ko", "yue", "auto"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(!ids.contains(&"fire-red-asr-v1"),
            "fire-red-asr-v1 should NOT be recommended for {} (too large, manual only)", lang);
    }
}

#[test]
fn test_fire_red_asr_model_paths() {
    let (enc, dec, tokens) = manager::fire_red_asr_model_paths("fire-red-asr-v1")
        .expect("paths should resolve");
    assert!(enc.ends_with("encoder.int8.onnx"));
    assert!(dec.ends_with("decoder.int8.onnx"));
    assert!(tokens.ends_with("tokens.txt"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --lib models::tests::test_fire_red_asr`
Expected: FAIL——"fire-red-asr-v1 must be registered" 和 "paths should resolve"

- [ ] **Step 3: 追加 `FIRE_RED_ASR_V1_FILES` 常量和 `ModelInfo`**

在 `src-tauri/src/models/registry.rs` 中的 Paraformer 区块之后（Moonshine 区块之前），新增 FireRedASR 常量块：

```rust
// ── FireRedASR models ───────────────────────────────────────────────────────

const FIRE_RED_ASR_V1_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "encoder.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-fire-red-asr-large-zh_en-2025-02-16/resolve/main/encoder.int8.onnx",
        size_bytes: 1_290_000_000,
        sha1: None,
    },
    ModelFile {
        relative_path: "decoder.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-fire-red-asr-large-zh_en-2025-02-16/resolve/main/decoder.int8.onnx",
        size_bytes: 445_000_000,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-fire-red-asr-large-zh_en-2025-02-16/resolve/main/tokens.txt",
        size_bytes: 71_400,
        sha1: None,
    },
];
```

在 `ALL_MODELS` 中 `paraformer-trilingual` 条目之后追加：

```rust
    ModelInfo {
        id: "fire-red-asr-v1",
        display_name: "FireRedASR Large v1",
        description: "优点：小红书开源的中文 ASR SOTA，AED 架构精度极高，中文 CER 逼近 2%\n缺点：体积 1.74 GB 极大，首次下载耗时长，推理速度不如非自回归模型",
        backend: BackendKind::FireRedAsr,
        total_size_bytes: 1_735_071_400,
        size_display: "1.74 GB",
        files: FIRE_RED_ASR_V1_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
```

- [ ] **Step 4: 在 `manager.rs` 加路径辅助**

在 `src-tauri/src/models/manager.rs` 的 `moonshine_model_paths` 之后追加：

```rust
pub fn fire_red_asr_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let encoder = dir.join("encoder.int8.onnx");
    let decoder = dir.join("decoder.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((encoder, decoder, tokens))
}
```

- [ ] **Step 5: 在 `lib.rs::load_stt_model` 把 FireRedAsr 占位换成真实调用**

在 `src-tauri/src/lib.rs` 顶部的 `use crate::stt::...` 行（约第 28 行），把 `MoonshineBackend` 后面补上 `FireRedAsrBackend`：

```rust
use crate::stt::{SharedTranscriber, whisper_backend::WhisperBackend, sensevoice_backend::SenseVoiceBackend, paraformer_backend::ParaformerBackend, moonshine_backend::MoonshineBackend, fire_red_asr_backend::FireRedAsrBackend};
```

找到 Task 1 Step 2 加的占位：

```rust
        BackendKind::FireRedAsr => {
            return Err(errors::AppError::Whisper(
                "FireRedAsr backend not yet wired up".to_string(),
            ));
        }
```

替换为：

```rust
        BackendKind::FireRedAsr => {
            let (encoder, decoder, tokens) =
                model_manager::fire_red_asr_model_paths(model_id)?;
            Box::new(FireRedAsrBackend::new(&encoder, &decoder, &tokens, model_id)?)
        }
```

`BackendKind::ZipformerCtc` 的占位暂不动，等 Task 6 处理。

- [ ] **Step 6: 运行测试 + 编译**

Run: `cd src-tauri && cargo test --lib models::tests`
Expected: PASS（新旧全通过）

Run: `cd src-tauri && cargo build --lib`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models/registry.rs src-tauri/src/models/manager.rs src-tauri/src/models/tests.rs src-tauri/src/lib.rs
git commit -m "feat(stt): 注册 FireRedASR v1 并接入 load_stt_model"
```

---

## Task 5: 新建 `ZipformerCtcBackend`

**Files:**
- Create: `src-tauri/src/stt/zipformer_ctc_backend.rs`
- Modify: `src-tauri/src/stt/mod.rs`

- [ ] **Step 1: 创建 `zipformer_ctc_backend.rs`**

Create `src-tauri/src/stt/zipformer_ctc_backend.rs`：

```rust
use std::path::Path;

use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineZipformerCtcModelConfig};

use crate::errors::AppError;
use crate::models::registry::BackendKind;
use crate::stt::TranscriberBackend;

pub struct ZipformerCtcBackend {
    recognizer: OfflineRecognizer,
    model_id: String,
}

unsafe impl Send for ZipformerCtcBackend {}
unsafe impl Sync for ZipformerCtcBackend {}

impl ZipformerCtcBackend {
    pub fn new(
        model_onnx_path: &Path,
        tokens_path: &Path,
        model_id: &str,
    ) -> Result<Self, AppError> {
        if !model_onnx_path.exists() {
            return Err(AppError::Whisper(format!(
                "Zipformer CTC model file not found: {}",
                model_onnx_path.display()
            )));
        }
        if !tokens_path.exists() {
            return Err(AppError::Whisper(format!(
                "Zipformer CTC tokens file not found: {}",
                tokens_path.display()
            )));
        }

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.zipformer_ctc = OfflineZipformerCtcModelConfig {
            model: Some(model_onnx_path.to_string_lossy().into_owned()),
        };
        config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
        config.model_config.num_threads = 4;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            AppError::Whisper("Failed to create Zipformer CTC recognizer".to_string())
        })?;

        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
        })
    }
}

impl TranscriberBackend for ZipformerCtcBackend {
    fn transcribe(&self, audio: &[f32], _language: &str) -> Result<String, AppError> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let stream = self.recognizer.create_stream();
        stream.accept_waveform(16000, audio);

        self.recognizer.decode(&stream);

        let result = stream
            .get_result()
            .ok_or_else(|| AppError::Whisper("Failed to get Zipformer CTC result".to_string()))?;

        Ok(result.text.trim().to_string())
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::ZipformerCtc
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}
```

- [ ] **Step 2: 在 `stt/mod.rs` 导出**

Edit `src-tauri/src/stt/mod.rs`。将：

```rust
pub mod fire_red_asr_backend;
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
```

替换为：

```rust
pub mod fire_red_asr_backend;
pub mod moonshine_backend;
pub mod paraformer_backend;
pub mod sensevoice_backend;
pub mod whisper_backend;
pub mod zipformer_ctc_backend;
```

- [ ] **Step 3: 验证编译**

Run: `cd src-tauri && cargo check --lib`
Expected: 编译通过，此时 `ZipformerCtcBackend` 仍未被使用（`load_stt_model` 中是占位 error）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/stt/zipformer_ctc_backend.rs src-tauri/src/stt/mod.rs
git commit -m "feat(stt): 新增 Zipformer CTC backend（离线中文专用）"
```

---

## Task 6: 注册 `zipformer-ctc-zh` + 接入 `load_stt_model`

**Files:**
- Modify: `src-tauri/src/models/registry.rs`
- Modify: `src-tauri/src/models/manager.rs`
- Modify: `src-tauri/src/models/tests.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 写测试**

追加到 `src-tauri/src/models/tests.rs` 末尾：

```rust
#[test]
fn test_zipformer_ctc_zh_registered() {
    let model = registry::get_model("zipformer-ctc-zh")
        .expect("zipformer-ctc-zh must be registered");
    assert_eq!(model.backend, registry::BackendKind::ZipformerCtc);
    assert_eq!(model.files.len(), 2, "should have model + tokens (bbpe.model not downloaded)");
    let paths: Vec<&str> = model.files.iter().map(|f| f.relative_path).collect();
    assert!(paths.contains(&"model.int8.onnx"));
    assert!(paths.contains(&"tokens.txt"));
}

#[test]
fn test_zipformer_ctc_zh_not_in_recommendation_pool() {
    for lang in ["zh", "en", "ja", "ko", "yue", "auto"] {
        let recs = registry::recommended_models_for_language(lang);
        let ids: Vec<&str> = recs.iter().map(|m| m.id).collect();
        assert!(!ids.contains(&"zipformer-ctc-zh"),
            "zipformer-ctc-zh should NOT be in recommendation pool for {}", lang);
    }
}

#[test]
fn test_zipformer_ctc_model_paths() {
    let (model, tokens) = manager::zipformer_ctc_model_paths("zipformer-ctc-zh")
        .expect("paths should resolve");
    assert!(model.ends_with("model.int8.onnx"));
    assert!(tokens.ends_with("tokens.txt"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --lib models::tests::test_zipformer_ctc`
Expected: FAIL——"zipformer-ctc-zh must be registered" 和 "paths should resolve"

- [ ] **Step 3: 追加 `ZIPFORMER_CTC_ZH_FILES` 常量和 `ModelInfo`**

在 `src-tauri/src/models/registry.rs` 中 Moonshine 区块之后新增：

```rust
// ── Zipformer CTC models ────────────────────────────────────────────────────

const ZIPFORMER_CTC_ZH_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "model.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/resolve/main/model.int8.onnx",
        size_bytes: 367_000_000,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03/resolve/main/tokens.txt",
        size_bytes: 13_400,
        sha1: None,
    },
];
```

在 `ALL_MODELS` 末尾（Moonshine 之后、最后一个 `]` 之前）追加：

```rust
    ModelInfo {
        id: "zipformer-ctc-zh",
        display_name: "Zipformer 中文 CTC",
        description: "优点：新一代 Kaldi 架构，离线中文专用，体积适中且推理快\n缺点：仅支持中文，精度略低于 SenseVoice / FireRedASR，作为轻量备选",
        backend: BackendKind::ZipformerCtc,
        total_size_bytes: 367_013_400,
        size_display: "350 MB",
        files: ZIPFORMER_CTC_ZH_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
```

- [ ] **Step 4: 在 `manager.rs` 加路径辅助**

在 `src-tauri/src/models/manager.rs` 的 `fire_red_asr_model_paths` 之后追加：

```rust
pub fn zipformer_ctc_model_paths(model_id: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = model_dir(model_id)?;
    let model = dir.join("model.int8.onnx");
    let tokens = dir.join("tokens.txt");
    Ok((model, tokens))
}
```

- [ ] **Step 5: 在 `lib.rs::load_stt_model` 把占位换成真实调用**

在 `src-tauri/src/lib.rs` 顶部的 `use crate::stt::...` 行，把 `FireRedAsrBackend` 后面补上 `ZipformerCtcBackend`：

```rust
use crate::stt::{SharedTranscriber, whisper_backend::WhisperBackend, sensevoice_backend::SenseVoiceBackend, paraformer_backend::ParaformerBackend, moonshine_backend::MoonshineBackend, fire_red_asr_backend::FireRedAsrBackend, zipformer_ctc_backend::ZipformerCtcBackend};
```

找到 Task 1 Step 2 加的占位：

```rust
        BackendKind::ZipformerCtc => {
            return Err(errors::AppError::Whisper(
                "ZipformerCtc backend not yet wired up".to_string(),
            ));
        }
```

替换为：

```rust
        BackendKind::ZipformerCtc => {
            let (model, tokens) = model_manager::zipformer_ctc_model_paths(model_id)?;
            Box::new(ZipformerCtcBackend::new(&model, &tokens, model_id)?)
        }
```

- [ ] **Step 6: 运行测试 + 编译**

Run: `cd src-tauri && cargo test --lib models::tests`
Expected: PASS（全部通过，包括粤语推荐、FireRedASR 路径、Zipformer 不在推荐池等）

Run: `cd src-tauri && cargo build --lib`
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models/registry.rs src-tauri/src/models/manager.rs src-tauri/src/models/tests.rs src-tauri/src/lib.rs
git commit -m "feat(stt): 注册 Zipformer 中文 CTC 并接入 load_stt_model"
```

---

## Task 7: 文档同步

**Files:**
- Modify: `docs/feature-add-stt-models.md`
- Modify: `docs/feature-model-switching.md`
- Modify: `docs/research-local-stt-models.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: 修订 `docs/feature-add-stt-models.md`**

改动：
1. 顶部状态 `设计中 📝` → `已实现 ✅`
2. "模型清单" 表中 Zipformer 行的"文件清单"去掉 `bbpe.model (255 KB)`，改为 `model.int8.onnx (367 MB) + tokens.txt (13 KB)`，总大小保持 `367 MB`
3. "风险与限制" 节的 "bbpe.model 的必要性" 子节替换为："**确认不需要 `bbpe.model`**。核查 `sherpa-onnx-1.12.34` crate 源码 `offline_asr.rs:255`，`OfflineZipformerCtcModelConfig` 仅暴露 `model: Option<String>` 字段。因此 `bbpe.model` 无处可传，不下载更干净（该文件 255 KB，用于 byte-level BPE 词表，当前 crate 未使用）。"
4. "验收标准" checkbox 全部改为 `[x]`（Task 9 完成后再勾选）——本 step 只做状态和技术细节更新，checkbox 留到 Task 9

- [ ] **Step 2: 修订 `docs/feature-model-switching.md`**

找到模型清单表或模型数量描述（如"9 个模型"），更新为 12 个模型，并补充三条新模型的简介：
- `paraformer-trilingual`：Paraformer 中英粤三语版，粤语识别唯一可用模型
- `fire-red-asr-v1`：FireRedASR 中文 SOTA，1.74 GB，追求极致精度
- `zipformer-ctc-zh`：Zipformer 中文 CTC 离线版，350 MB 轻量备选

具体行定位用 `grep -n "9 个" docs/feature-model-switching.md` 找起始行，并保持原文档风格。

- [ ] **Step 3: 修订 `docs/research-local-stt-models.md`**

找到 "方案 C：多模型策略（已实现 ✅）" 节，把"四引擎架构（Whisper / SenseVoice / Paraformer / Moonshine）+ 9 个模型"更新为"四引擎架构 + 12 个模型（新增 FireRedASR 引擎）"。补一段说明 2026-04 的扩充。

- [ ] **Step 4: 修订 `CLAUDE.md` Documentation Map**

在 Documentation Map 表格中新增一行：

```markdown
| [docs/feature-add-stt-models.md](docs/feature-add-stt-models.md) | 新增三款 STT 模型：FireRedASR v1 / Paraformer 三语 / Zipformer 中文 CTC | 2026-04-19 |
```

同时把 `feature-model-switching.md` 和 `research-local-stt-models.md` 的"最后校验"日期更新为 `2026-04-19`。

- [ ] **Step 5: Commit**

```bash
git add docs/feature-add-stt-models.md docs/feature-model-switching.md docs/research-local-stt-models.md CLAUDE.md
git commit -m "docs: 同步三款新 STT 模型相关文档"
```

---

## Task 8: 全量验证

**Files:**
- 无文件改动，仅验证

- [ ] **Step 1: 跑 Rust 测试**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部测试通过，包括新加的 `test_paraformer_trilingual_registered`、`test_paraformer_trilingual_recommended_for_cantonese`、`test_fire_red_asr_v1_registered`、`test_fire_red_asr_v1_not_in_recommendation_pool`、`test_fire_red_asr_model_paths`、`test_zipformer_ctc_zh_registered`、`test_zipformer_ctc_zh_not_in_recommendation_pool`、`test_zipformer_ctc_model_paths`

- [ ] **Step 2: 跑 Rust 编译**

Run: `cd src-tauri && cargo build --lib`
Expected: 编译通过，无 warning 或仅常规 unused_variables 类

- [ ] **Step 3: 跑前端类型检查 + 构建**

Run: `pnpm build`
Expected: tsc 零报错 + vite build 成功

- [ ] **Step 4: 勾选 feature doc 验收标准**

Edit `docs/feature-add-stt-models.md` 的 "验收标准" 节：

将能验证的 checkbox 勾选：
- [x] `cargo test --lib` 全部通过
- [x] 新增三个模型在 Settings → Models 列表中可见（registry 驱动，编译通过即自动生效）
- [x] 粤语（yue）在语言推荐中出现 paraformer-trilingual（test_paraformer_trilingual_recommended_for_cantonese 通过）

剩余三条（实际下载+转写测试）属于运行时冒烟验证，需要用户在 `pnpm tauri dev` 实机操作，本 plan 完成后由用户自行验证，**不在本次 commit 范围**。文档中把这三条保持 `[ ]` 未勾选，作为用户手动验证清单。

- [ ] **Step 5: Commit 验收标准更新**

```bash
git add docs/feature-add-stt-models.md
git commit -m "docs: 勾选 feature-add-stt-models.md 可自动验证的验收标准"
```

- [ ] **Step 6: 报告完成**

最终向用户汇报：
1. 已完成 8 个 commit（枚举扩展+占位 → Paraformer 三语 → FireRedASR backend → FireRedASR 注册 → Zipformer backend → Zipformer 注册 → 文档同步 → 验收标准）
2. 所有自动测试通过
3. 剩余手动验证清单：让用户在 Settings 页下载这三个模型，分别切换并录音测试
