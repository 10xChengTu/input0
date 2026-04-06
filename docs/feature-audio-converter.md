# Feature: Audio Converter Module

## 状态
已完成 ✅

## 需求分析
实现音频转换模块，将 cpal 捕获的原始音频转换为 Whisper 所需格式：
- Whisper 要求：单声道、f32 格式、16000 Hz 采样率
- cpal 典型输出：立体声、i16 或 f32、48kHz 或 44.1kHz

## 技术方案

### 模块结构
- `src-tauri/src/audio/converter.rs` — 生产代码
- `src-tauri/src/audio/tests.rs` — 测试代码（独立文件模块）

### 公共 API
```rust
pub fn stereo_to_mono(samples: &[f32]) -> Vec<f32>
pub fn i16_to_f32(samples: &[i16]) -> Vec<f32>
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, AppError>
pub fn prepare_for_whisper(samples: &[f32], channels: u16, sample_rate: u32) -> Result<Vec<f32>, AppError>
```

### 关键实现决策
- **resample**：使用 `rubato::SincFixedIn`，参数 sinc_len=256、f_cutoff=0.95、Linear 插值
- **同率优化**：from_rate == to_rate 时直接返回输入，跳过 rubato 构建开销
- **空输入**：直接返回空 Vec，不进入 rubato（避免构建失败）
- **i16→f32**：除以 32768.0（非 32767.0），保持 MIN 精确为 -1.0
- **奇数长度 stereo**：使用 `chunks_exact(2)` 自动截断末尾不完整的帧

## 测试覆盖（24个测试全部通过）
- stereo_to_mono: 6个测试（基本、空、单对、相同声道、奇数长度、静音）
- i16_to_f32: 5个测试（基本、最大值、最小值、零、空）
- resample: 8个测试（同率、48k→16k、44.1k→16k、静音保持、零from_rate错误、零to_rate错误、空输入、短输入）
- prepare_for_whisper: 5个测试（单声道16k直通、立体声48k、单声道48k、立体声16k、空输入）

## TDD 流程记录
1. 先写测试（tests.rs）
2. 测试编译失败（converter.rs 为空）
3. 实现 converter.rs
4. 24/24 测试通过，cargo build 成功
