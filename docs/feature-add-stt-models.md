# Feature: 新增三款 STT 模型

## 状态：设计中 📝

## 需求背景

用户反馈现有模型矩阵中 SenseVoice 只有 Small 一款，希望扩充 sherpa-onnx 生态下其他中文/多语言模型。经调研，SenseVoice Large 未开源（只有论文 benchmark），但同生态下有三类可加模型：

1. **FireRedASR**——小红书开源，当前中文 ASR 的 SOTA 之一
2. **Paraformer 三语版**——阿里 Paraformer 家族的中/英/粤三语版本（现有 `paraformer-zh` 是单中文）
3. **Zipformer 中文 CTC**——新一代 Kaldi (k2-fsa) 社区出品，offline 中文专用

这三款均可通过 `sherpa-onnx` 1.12 crate（已集成）直接加载，与现有 SenseVoice / Paraformer / Moonshine 走同一套 `OfflineRecognizer` 路径。

---

## 目标

在不改动现有下载器 / Pipeline / 前端的前提下，向模型注册表新增三个可选 STT 模型。用户可在 Settings → Models 中按需下载并切换。

### 非目标

- 不支持 sherpa-onnx GitHub Releases 的 tar.bz2 压缩包下载（下载器仅支持 HuggingFace 单文件）。因此只收录 HuggingFace 有单文件仓库的模型。
- 不引入 FireRedASR v2（2026-02 发布，尚无 sherpa-onnx 适配的 HF 单文件仓库；v1 已有 HF 仓库）。
- 不改动 Pipeline 状态机、前端 UI、配置结构。

---

## 模型清单

| 模型 ID | 架构 | 后端 | 文件清单 | 总大小 | 语言 |
|---------|------|------|---------|--------|------|
| `fire-red-asr-v1` | AED | `OfflineFireRedAsrModelConfig` | `encoder.int8.onnx` (1.29 GB) + `decoder.int8.onnx` (445 MB) + `tokens.txt` (71 KB) | **1.74 GB** | 中 + 英 |
| `paraformer-trilingual` | Paraformer | `OfflineParaformerModelConfig`（复用） | `model.int8.onnx` (245 MB) + `tokens.txt` (119 KB) | **245 MB** | 中 + 英 + 粤 |
| `zipformer-ctc-zh` | Zipformer CTC | `OfflineZipformerCtcModelConfig` | `model.int8.onnx` (367 MB) + `tokens.txt` (13 KB) + `bbpe.model` (255 KB) | **367 MB** | 中 |

### 下载来源（HuggingFace）

- FireRedASR v1：`csukuangfj/sherpa-onnx-fire-red-asr-large-zh_en-2025-02-16`
- Paraformer 三语：`csukuangfj/sherpa-onnx-paraformer-trilingual-zh-cantonese-en`
- Zipformer CTC：`csukuangfj/sherpa-onnx-zipformer-ctc-zh-int8-2025-07-03`

### 语言推荐 (`best_for_languages`)

| 模型 | 推荐语言 | 推荐理由 |
|------|---------|---------|
| `fire-red-asr-v1` | 不加入推荐池 | 体积过大（1.74GB），定位"追求极致精度"手动选择 |
| `paraformer-trilingual` | `yue`（粤语） | 粤语目前无推荐模型，此款是唯一明确支持粤语的 |
| `zipformer-ctc-zh` | 不加入推荐池 | 作为可选备选项，中文 zh 主推仍是 SenseVoice |

> 保持中文默认推荐仍为 SenseVoice（推理快、自带标点），避免推荐池被稀释。

---

## 技术方案

### 1. `src-tauri/src/models/registry.rs`

**`BackendKind` 枚举扩展**：

```rust
pub enum BackendKind {
    Whisper,
    SenseVoice,
    Paraformer,
    Moonshine,
    FireRedAsr,    // NEW
    ZipformerCtc,  // NEW
}
```

**新增 3 个 `ModelFile` 常量块 + 3 条 `ModelInfo`**（追加到 `ALL_MODELS` 末尾，不动现有顺序）。描述文案沿用现有"优点/缺点"格式。

### 2. `src-tauri/src/models/manager.rs`

新增两个路径辅助函数（Paraformer 三语复用现有 `paraformer_model_paths`，因为文件名相同）：

```rust
pub fn fire_red_asr_model_paths(model_id: &str)
    -> Result<(PathBuf, PathBuf, PathBuf), AppError> { /* encoder, decoder, tokens */ }

pub fn zipformer_ctc_model_paths(model_id: &str)
    -> Result<(PathBuf, PathBuf, PathBuf), AppError> { /* model, tokens, bbpe */ }
```

### 3. `src-tauri/src/stt/` 新增两个 backend

**`fire_red_asr_backend.rs`**（参照 `paraformer_backend.rs`，但接三个路径）：

```rust
config.model_config.fire_red_asr = OfflineFireRedAsrModelConfig {
    encoder: Some(encoder_path.to_string_lossy().into_owned()),
    decoder: Some(decoder_path.to_string_lossy().into_owned()),
};
config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
```

**`zipformer_ctc_backend.rs`**：

```rust
config.model_config.zipformer_ctc = OfflineZipformerCtcModelConfig {
    model: Some(model_onnx_path.to_string_lossy().into_owned()),
};
config.model_config.tokens = Some(tokens_path.to_string_lossy().into_owned());
// bbpe.model 传递方式（bpe_vocab 字段或 model_config 上的 bpe_vocab）
// —— 实现时以 sherpa-onnx 1.12 crate 的实际字段为准
```

> 实现时需确认 sherpa-onnx 1.12 中 `OfflineZipformerCtcModelConfig` 字段名以及 `bbpe.model` 应通过哪个字段传入。若 crate 未暴露该字段，则先不传（Zipformer CTC 通常 tokens.txt 足以，bbpe 用于后处理解码纠正）。

### 4. `src-tauri/src/stt/mod.rs`

```rust
pub mod fire_red_asr_backend;  // NEW
pub mod zipformer_ctc_backend; // NEW
```

### 5. `src-tauri/src/lib.rs::load_stt_model`

新增 2 条 `BackendKind` 分支：

```rust
BackendKind::FireRedAsr => {
    let (enc, dec, tokens) = model_manager::fire_red_asr_model_paths(model_id)?;
    Box::new(FireRedAsrBackend::new(&enc, &dec, &tokens, model_id)?)
}
BackendKind::ZipformerCtc => {
    let (model, tokens, bbpe) = model_manager::zipformer_ctc_model_paths(model_id)?;
    Box::new(ZipformerCtcBackend::new(&model, &tokens, &bbpe, model_id)?)
}
```

---

## 数据流

与现有 SenseVoice / Paraformer / Moonshine 完全一致：

1. 用户在 Settings 选中新模型 → `download_model` command → `manager::download_model` 逐文件下载到 `~/Library/Application Support/com.input0.app/models/<model_id>/`
2. 用户切换模型 → `switch_model` command → `load_stt_model` → 构造对应 backend → 塞进 `ManagedTranscriber`
3. Pipeline 调用 `ManagedTranscriber::transcribe(audio, language)` → 动态分发到 `FireRedAsrBackend::transcribe` 或 `ZipformerCtcBackend::transcribe`

---

## 前端影响

**零改动**。Settings 页的模型列表由 `list_models` command 从 registry 驱动，新增 ModelInfo 会自动出现在 UI 中。图标 / 描述 / 语言标签全走现有模板。

---

## 风险与限制

### FireRedASR v1 体积

1.74 GB 是目前第二大模型（仅次于 Whisper Large v3 的 2.9 GB）。在 description 中明确标注"体积较大，适合追求最高中文精度的用户"，让用户有心理预期。首次下载对网速敏感。

### bbpe.model 的必要性

Zipformer CTC 模型目录里有一个 `bbpe.model`（byte-level BPE 词表，255 KB）。sherpa-onnx 官方 Python 示例把它传给 `bpe_vocab` 字段，但 sherpa-rs 1.12 是否暴露此字段待验证。实现时分两步：
1. 先不传 `bpe_vocab`，测一下能不能直接转写中文
2. 如果中文输出不对（出现 byte-level 乱码），再查 crate 字段并传入

若 1.12 crate 确实缺字段，临时方案是下载但不使用 `bbpe.model`；长期方案是升级 sherpa-onnx crate。

### FireRedASR v2 暂不上

v2 (2026-02 发布) 主流 sherpa-onnx 分发只在 GitHub Releases `.tar.bz2`，HuggingFace 单文件版未发布。等未来有 HF 单文件仓库后再加。

---

## 实现计划

1. **Step 1**：扩展 `BackendKind` 枚举 + 3 条 `ModelInfo` 注册。运行 `cargo build` 验证编译。
2. **Step 2**：新增 `fire_red_asr_backend.rs` + `zipformer_ctc_backend.rs` 两个文件，并在 `stt/mod.rs` 导出。
3. **Step 3**：`manager.rs` 加 2 个 path helper。
4. **Step 4**：`lib.rs::load_stt_model` 加 2 条 match 分支。
5. **Step 5**：`cargo test --lib` 通过；`cargo build` 通过；前端 `pnpm build` 通过。
6. **Step 6**：手动冒烟测试（可在下一次开发构建时验证）——下载任一新模型、切换、录音转写。
7. **Step 7**：更新文档
   - 本文档状态：设计中 → 已实现
   - `docs/feature-model-switching.md`：模型清单从 9 款 → 12 款
   - `docs/research-local-stt-models.md`：方案 C 部分更新模型矩阵
   - `CLAUDE.md` Documentation Map：新增本文档条目，更新相关文档校验日期

---

## 验收标准

- [ ] `cargo test --lib` 全部通过
- [ ] 新增三个模型在 Settings → Models 列表中可见，size 显示正确
- [ ] 粤语（`yue`）在语言推荐中出现 `paraformer-trilingual`
- [ ] FireRedASR v1 下载完成后可成功加载并转写中文
- [ ] Paraformer Trilingual 下载完成后可成功转写中文并支持粤语
- [ ] Zipformer CTC 下载完成后可成功转写中文
- [ ] 所有新模型的切换 / 卸载 / 删除与现有模型行为一致
