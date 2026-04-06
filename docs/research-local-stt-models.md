# 本地语音转文字（STT）模型调研报告

## 状态：已完成 ✅

## 调研背景

Input0 当前使用 **whisper-rs v0.14**（基于 whisper.cpp）+ **ggml-base.bin** 模型（~142MB），启用 Metal GPU 加速。实际使用中存在以下问题：

- 使用的是 Whisper **base** 级别模型，参数量仅 74M，准确率有限
- 中文识别存在简繁体不稳定问题（已通过 initial_prompt 缓解）
- 对中文语音识别的整体效果不够理想

本报告调研 2024-2026 年间主流本地 STT 模型，为模型升级提供选型依据。

---

## 一、当前方案分析

### Whisper Base 模型的局限

| 指标 | Whisper Base | Whisper Large-v3 |
|------|-------------|------------------|
| 参数量 | 74M | 1.55B |
| 模型大小 | ~142MB | ~3.1GB |
| 英文 WER | ~10-12% | ~5-6% |
| 中文 CER | 较高 | 明显更低 |
| 推理速度（Apple Silicon） | 非常快 | 较慢但可接受 |

**核心问题**：当前使用的 base 模型是 Whisper 系列中偏小的模型，准确率远低于 large 级别。仅通过升级模型大小就能获得显著的识别效果提升。

---

## 二、主流本地 STT 模型横向对比

### 2.1 综合对比表

| 模型 | 开发者 | 参数量 | 英文 WER | 中文能力 | 推理速度 (RTFx) | macOS/Apple Silicon | 许可证 |
|------|--------|--------|----------|----------|----------------|---------------------|--------|
| **Whisper Large-v3** | OpenAI | 1.55B | ~5-6% | 好 | ~50x | ✅ whisper.cpp/Metal | MIT |
| **Whisper Large-v3-turbo** | OpenAI | 809M | ~6-7% | 好 | ~216x | ✅ whisper.cpp/Metal | MIT |
| **SenseVoice-Small** | 阿里 FunAudioLLM | 234M | 良好 | **极佳** | ~**750x**（比Whisper快5-15倍） | ✅ ONNX/sherpa-onnx | MIT |
| **Paraformer-zh** | 阿里 FunASR | 220M | 不适用 | **极佳** | ~300x | ✅ ONNX/sherpa-onnx | MIT |
| **Parakeet TDT 0.6B v2** | NVIDIA | 600M | **1.69%** | 不支持中文 | ~**3386x** | ⚠️ 需 NVIDIA GPU | CC-BY-4.0 |
| **Canary Qwen 2.5B** | NVIDIA | 2.5B | **5.63%** | 支持（含中文） | ~418x | ⚠️ 需 NVIDIA GPU | Apache 2.0 |
| **Moonshine** | Useful Sensors | 31M/262M | ~8-10% | 不支持 | 极快（边缘设备） | ✅ ONNX | MIT |
| **Vosk** | Alpha Cephei | 多种 | ~10-15% | 支持 | 快 | ✅ 原生支持 | Apache 2.0 |
| **sherpa-onnx** | k2-fsa (新一代 Kaldi) | 多种模型 | 取决于模型 | ✅ 多模型 | 快 | ✅ 原生支持，含 Rust API | Apache 2.0 |

> **RTFx**：Real-Time Factor eXpressed，表示每秒计算时间能处理多少秒音频。值越大越快。

### 2.2 各模型详细分析

---

### 🏆 SenseVoice-Small（强烈推荐）

**开发者**：阿里巴巴 FunAudioLLM 团队
**GitHub**：https://github.com/FunAudioLLM/SenseVoice（⭐ 10k+）

#### 核心优势

- **中文识别效果极佳**：在 AISHELL-1、AISHELL-2、Wenetspeech 等中文基准测试中，CER（字符错误率）**显著优于 Whisper Large-v3**
- **推理速度极快**：比 Whisper Large-v3 快 **5-15 倍**（非自回归架构，一次性输出）
- **模型极小**：仅 ~234M 参数，ONNX 量化后模型文件约 200-300MB
- **多语言支持**：中文、英文、日文、韩文、粤语等 50+ 语言
- **附加能力**：支持情感识别、语音事件检测（音乐、掌声、笑声等）
- **已被 sherpa-onnx 集成**：可直接通过 sherpa-onnx 使用，支持多平台部署

#### 架构特点

SenseVoice 采用**非自回归（Non-Autoregressive, NAR）**架构，与 Whisper 的自回归解码不同：
- Whisper 逐 token 生成文本，速度受限于序列长度
- SenseVoice 一次性并行输出所有 token，推理速度与音频长度几乎无关

#### 中文基准测试对比

| 测试集 | SenseVoice-Small | Whisper Large-v3 | 胜出 |
|--------|-----------------|------------------|------|
| AISHELL-1 | **2.85%** CER | 8.4% CER | SenseVoice |
| AISHELL-2 | **3.48%** CER | 5.7% CER | SenseVoice |
| Wenetspeech (test_net) | **7.04%** | 9.7% | SenseVoice |
| 普通话日常对话 | 优秀 | 良好 | SenseVoice |

> 数据来源：SenseVoice 官方 GitHub 和社区基准测试

#### 集成方式

- **sherpa-onnx（推荐）**：通过 sherpa-onnx 的 Rust/C API 直接调用，跨平台支持好
- **ONNX Runtime**：导出为 ONNX 格式，可用 onnxruntime-rs 在 Rust 中调用
- **CoreML**：可转换为 CoreML 模型在 Apple Silicon 上加速

#### 局限

- 不支持流式（streaming）识别，仅支持离线（非实时）转写
- 英文准确率略低于 Whisper Large-v3（但差距不大）
- 较新的模型，社区生态不如 Whisper 成熟

---

### Whisper Large-v3-turbo（稳妥升级选择）

**开发者**：OpenAI
**发布时间**：2024 年 10 月

#### 核心优势

- **模型更小更快**：将解码器层从 32 层减少到 4 层，参数量从 1.55B 降至 809M
- **速度提升 5.4 倍**：相比 Whisper Large-v3
- **准确率接近**：与 Large-v2 准确率相当，略低于 Large-v3
- **完美兼容现有代码**：whisper.cpp / whisper-rs 已完整支持
- **99 种语言**：语言覆盖最广
- **MIT 许可证**：最宽松的开源许可

#### 升级成本

- **代码改动极小**：仅需更换模型文件（`ggml-large-v3-turbo.bin`，约 1.6GB）
- whisper-rs 已原生支持，无需修改代码逻辑
- Metal 加速在 Apple Silicon 上表现良好

#### 局限

- 中文效果不如 SenseVoice
- 自回归架构，推理速度受限于序列长度
- 模型文件相对较大（~1.6GB）

---

### Paraformer-zh（中文专用版本）

**开发者**：阿里巴巴 FunASR 团队
**GitHub**：https://github.com/modelscope/FunASR（⭐ 15k+）

> 注：Paraformer 家族包含 Paraformer-Large（通用版）和 Paraformer-zh（中文专用版）。本项目实际集成的是 **Paraformer-zh**（通过 sherpa-onnx），因其中文识别更为精准。

#### 核心优势

- **中文识别顶级**：在中文 ASR 任务中性能接近 SenseVoice，针对中文场景深度优化
- **支持流式识别**：Paraformer 家族有流式版本，适合实时转写
- **模型小巧**：约 220M 参数，ONNX 模型文件约 232MB
- **已集成到 sherpa-onnx**：跨平台部署方便，已作为 `paraformer-zh` 加入本项目模型注册表

#### 局限

- **仅支持中文**：paraformer-zh 专为中文设计，不支持其他语言
- 文档以中文为主，国际社区较小

---

### NVIDIA Parakeet TDT 0.6B v2

**开发者**：NVIDIA

#### 核心优势

- **英文 WER 最低**：在 LibriSpeech 上仅 **1.69%**，Open ASR Leaderboard 排名前列
- **极快推理**：RTFx 高达 3386（每秒处理 3386 秒音频）
- **仅 600M 参数**：参数效率极高

#### 局限

- **不支持中文**：仅支持英文
- **依赖 NVIDIA GPU**：基于 NeMo 框架，需要 CUDA
- **不适合 macOS**：无 Metal/CoreML 支持，不适合 Apple Silicon 部署

> 结论：对于本项目（macOS + 中文为主）**不适用**。

---

### NVIDIA Canary Qwen 2.5B

**开发者**：NVIDIA

#### 核心优势

- **多语言准确率高**：Open ASR Leaderboard 排名第一（WER 5.63%）
- **支持中文**：包含中日韩语言支持
- **混合架构（SALM）**：结合 ASR + LLM 能力

#### 局限

- **依赖 NVIDIA GPU**：需要 CUDA，不支持 macOS 原生部署
- **参数量大**：2.5B 参数，需要较大显存
- 部署复杂度较高

> 结论：对于本项目**不适用**（NVIDIA GPU 依赖）。

---

### Moonshine

**开发者**：Useful Sensors
**GitHub**：https://github.com/usefulsensors/moonshine

#### 核心优势

- **极小极快**：专为边缘设备设计，Base 模型仅 31M 参数
- **超低延迟**：适合实时场景
- **支持 ONNX**：可在 Apple Silicon 上运行

#### 局限

- **仅支持英语**：不支持中文
- 准确率低于 Whisper Large 系列
- 社区生态较小

> 结论：不支持中文，但作为英文专用选项仍有价值。已通过 sherpa-onnx 集成为 `moonshine-base-en`，供英文用户使用——模型体积约 274MB、延迟极低，适合对英文识别速度有要求的场景。

---

### sherpa-onnx（运行时框架，非模型）

**开发者**：k2-fsa（新一代 Kaldi）
**GitHub**：https://github.com/k2-fsa/sherpa-onnx（⭐ 6k+）

#### 核心价值

sherpa-onnx 不是一个 STT 模型，而是一个**跨平台推理框架**，可以运行多种 STT 模型：

- ✅ Whisper (tiny/base/small/medium/large)
- ✅ SenseVoice
- ✅ Paraformer
- ✅ Zipformer (CTC/Transducer)
- ✅ 更多模型...

#### 技术优势

- **Rust API 可用**：提供 Rust bindings（crate: sherpa-rs）
- **Apple Silicon 原生支持**：支持 CoreML 和 Metal 加速
- **轻量级**：基于 ONNX Runtime，无需 Python 环境
- **流式 + 非流式**：两种模式都支持
- **模型动物园**：预转换的 ONNX 模型直接下载使用

#### 集成方案

```
sherpa-onnx (框架)
  ├── SenseVoice-Small (ONNX) → 中文最佳
  ├── Whisper Large-v3-turbo (ONNX) → 多语言
  ├── Paraformer-zh (ONNX) → 中文专用
  ├── Moonshine Base (ONNX) → 英文轻量
  └── 更多模型...
```

这种架构允许按需切换模型，而无需更换底层推理代码。

---

### Vosk

**开发者**：Alpha Cephei

#### 概述

- 老牌离线 ASR 方案，模型小（50-200MB）
- 支持中文，但准确率明显低于现代模型（Whisper、SenseVoice 等）
- 已基本被新一代模型超越

> 结论：**不推荐**，准确率已落后于主流。

---

## 三、适用于本项目的升级方案

### 方案对比

| 方案 | 中文效果 | 英文效果 | 改造成本 | 模型大小 | 推荐度 |
|------|---------|---------|---------|---------|--------|
| **A. 升级到 Whisper Large-v3-turbo** | 好 | 好 | ⭐ 极低（仅换模型文件） | ~1.6GB | ⭐⭐⭐⭐ |
| **B. 切换到 SenseVoice（via sherpa-onnx）** | **极佳** | 良好 | ⭐⭐⭐ 中等（换推理框架） | ~300MB | ⭐⭐⭐⭐⭐ |
| **C. 多模型策略** | 极佳 | 好 | ⭐⭐⭐⭐ 较高 | ~1.9GB+ | ⭐⭐⭐⭐⭐ |

---

### 方案 A：升级到 Whisper Large-v3-turbo（最低成本）

**改动量**：仅下载新模型文件，代码零改动

```bash
# 下载 large-v3-turbo 模型（约 1.6GB）
curl -L -o src-tauri/resources/ggml-large-v3-turbo.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/models/ggml-large-v3-turbo.bin
```

**预期效果**：
- 英文 WER 从 ~10-12% 降至 ~6-7%
- 中文识别准确率显著提升
- 推理速度仍然很快（turbo 版本比 large-v3 快 5 倍）
- 完全兼容现有 whisper-rs 代码

**适合场景**：快速验证效果提升，风险最低。

---

### 方案 B：切换到 SenseVoice + sherpa-onnx（最佳中文效果）

**改动量**：替换推理引擎，从 whisper-rs 迁移到 sherpa-rs

**核心变更**：
1. 替换依赖：`whisper-rs` → `sherpa-rs`（或直接调用 sherpa-onnx C API）
2. 下载 SenseVoice-Small ONNX 模型（约 200-300MB）
3. 调整 transcriber.rs 中的推理逻辑

**预期效果**：
- 中文 CER 降低约 50-70%（对比当前 base 模型）
- 推理速度可能更快（非自回归架构）
- 模型文件更小（~300MB vs 当前 142MB 的 base 模型）

**适合场景**：主要面向中文用户，追求最佳中文识别体验。

---

### 方案 C：多模型策略（已实现 ✅）

根据用户设置的语言，通过 `suggest_model_switch` 自动推荐最优模型：
- `language = "zh"` / `"ja"` / `"ko"` → 推荐 SenseVoice-Small（中日韩最佳）
- `language = "en"` → 推荐 Whisper Large-v3-turbo（英文最佳）
- `language = "auto"` / `"fr"` / `"de"` / `"es"` / `"ru"` → 推荐 Whisper Large-v3（多语言通用）
- 其他未命中的语言 → 回退到 Whisper Base

此外，Paraformer-zh（中文专用）和 Moonshine Base（英文轻量）作为可选下载项供用户手动选择。

**当前实现**：已通过四引擎架构（Whisper / SenseVoice / Paraformer / Moonshine）+ 9 个模型注册表完成，详见 `docs/feature-model-switching.md`。

---

## 四、推荐策略

### 短期（立即可做）

**升级到 Whisper Large-v3-turbo**（方案 A）

- 零代码改动，仅替换模型文件
- 立刻获得显著的准确率提升
- 风险为零

### 中期（1-2 周）

**引入 SenseVoice via sherpa-onnx**（方案 B）

- 为中文用户提供最佳体验
- sherpa-onnx 提供 Rust bindings，集成可行
- 可以与 Whisper 并存，按语言切换

### 长期展望

- 关注 **Whisper Large-v4** 或 OpenAI 下一代模型的发布
- 关注 **SenseVoice-Large** 的开源进展（目前仅 Small 开源）
- 考虑 **流式识别**需求：Paraformer-zh 已集成，其流式版本可按需启用

---

## 五、参考资源

| 资源 | 链接 |
|------|------|
| SenseVoice GitHub | https://github.com/FunAudioLLM/SenseVoice |
| FunASR GitHub | https://github.com/modelscope/FunASR |
| sherpa-onnx GitHub | https://github.com/k2-fsa/sherpa-onnx |
| whisper.cpp GitHub | https://github.com/ggerganov/whisper.cpp |
| Open ASR Leaderboard | https://huggingface.co/spaces/hf-audio/open_asr_leaderboard |
| Modal: Top OSS STT 2025 | https://modal.com/blog/open-source-stt |
| Moonshine GitHub | https://github.com/usefulsensors/moonshine |
| NVIDIA Parakeet | https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2 |
| NVIDIA Canary Qwen | https://huggingface.co/nvidia/canary-qwen-2.5b |

---

## 六、结论

**当前最大的改进空间在于模型本身**——从 Whisper base 升级到更大的模型或更适合中文的模型，将带来质的飞跃。

**已实施方案**（方案 C — 多模型策略 ✅）：
- 四引擎架构：Whisper / SenseVoice / Paraformer / Moonshine
- 9 个可下载模型，覆盖不同语言、精度、速度需求
- 智能推荐系统：根据用户语言设置自动推荐最优模型
- 详见 `docs/feature-model-switching.md`
