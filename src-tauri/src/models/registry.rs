use serde::Serialize;

/// Identifies which inference backend a model uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    Whisper,
    SenseVoice,
    Paraformer,
    Moonshine,
    FireRedAsr,
    ZipformerCtc,
}

/// A single file that must be downloaded for a model.
#[derive(Debug, Clone)]
pub struct ModelFile {
    /// Relative path inside the model's storage directory (e.g. "model.int8.onnx").
    pub relative_path: &'static str,
    /// Full download URL.
    pub url: &'static str,
    /// Expected file size in bytes (for progress calculation).
    pub size_bytes: u64,
    /// SHA1 hex digest for integrity check (from HuggingFace).
    /// Using SHA1 because HuggingFace provides SHA1 in their API.
    pub sha1: Option<&'static str>,
}

/// Static metadata describing a downloadable STT model.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Unique model identifier used in config (e.g. "whisper-base").
    pub id: &'static str,
    /// Human-readable display name.
    pub display_name: &'static str,
    /// Short description shown in UI.
    pub description: &'static str,
    /// Inference backend required.
    pub backend: BackendKind,
    /// Total download size in bytes (sum of all files).
    pub total_size_bytes: u64,
    /// Human-readable size string (e.g. "142 MB").
    pub size_display: &'static str,
    /// Files to download.
    pub files: &'static [ModelFile],
    /// Languages this model excels at (ISO 639-1 codes).
    pub best_for_languages: &'static [&'static str],
    /// Short recommendation reason shown when this model is suggested.
    pub recommendation_reason: &'static str,
}

/// Serializable model info sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfoDto {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub backend: BackendKind,
    pub total_size_bytes: u64,
    pub size_display: String,
    pub best_for_languages: Vec<String>,
    pub is_downloaded: bool,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// Model Registry — all supported models
// ---------------------------------------------------------------------------

// ── Whisper models ──────────────────────────────────────────────────────────

const WHISPER_BASE_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-base.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    size_bytes: 148_000_000, // ~142 MiB
    sha1: Some("465707469ff3a37a2b9b8d8f89f2f99de7299dac"),
}];

const WHISPER_SMALL_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-small.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    size_bytes: 488_636_416,
    sha1: None,
}];

const WHISPER_MEDIUM_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-medium.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    size_bytes: 1_533_774_848,
    sha1: None,
}];

const WHISPER_LARGE_V3_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-large-v3.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    size_bytes: 3_094_623_232,
    sha1: None,
}];

const WHISPER_LARGE_V3_TURBO_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-large-v3-turbo.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
    size_bytes: 1_620_000_000, // ~1.5 GiB
    sha1: Some("4af2b29d7ec73d781377bfd1758ca957a807e941"),
}];

const WHISPER_LARGE_V3_TURBO_Q5_FILES: &[ModelFile] = &[ModelFile {
    relative_path: "ggml-large-v3-turbo-q5_0.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
    size_bytes: 574_000_000, // ~547 MiB
    sha1: Some("e050f7970618a659205450ad97eb95a18d69c9ee"),
}];

// ── SenseVoice models ───────────────────────────────────────────────────────

const SENSEVOICE_SMALL_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "model.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx",
        size_bytes: 239_000_000,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt",
        size_bytes: 316_000,
        sha1: None,
    },
];

// ── Paraformer models ───────────────────────────────────────────────────────

const PARAFORMER_ZH_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "model.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2024-03-09/resolve/main/model.int8.onnx",
        size_bytes: 227_330_205,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2024-03-09/resolve/main/tokens.txt",
        size_bytes: 75_354,
        sha1: None,
    },
];

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

// ── Moonshine models ────────────────────────────────────────────────────────

const MOONSHINE_BASE_EN_FILES: &[ModelFile] = &[
    ModelFile {
        relative_path: "preprocess.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-base-en-int8/resolve/main/preprocess.onnx",
        size_bytes: 14_077_290,
        sha1: None,
    },
    ModelFile {
        relative_path: "encode.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-base-en-int8/resolve/main/encode.int8.onnx",
        size_bytes: 50_311_494,
        sha1: None,
    },
    ModelFile {
        relative_path: "uncached_decode.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-base-en-int8/resolve/main/uncached_decode.int8.onnx",
        size_bytes: 122_120_451,
        sha1: None,
    },
    ModelFile {
        relative_path: "cached_decode.int8.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-base-en-int8/resolve/main/cached_decode.int8.onnx",
        size_bytes: 99_983_837,
        sha1: None,
    },
    ModelFile {
        relative_path: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-moonshine-base-en-int8/resolve/main/tokens.txt",
        size_bytes: 436_688,
        sha1: None,
    },
];

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

// ── ALL_MODELS registry ─────────────────────────────────────────────────────

pub static ALL_MODELS: &[ModelInfo] = &[
    // --- Whisper family ---
    ModelInfo {
        id: "whisper-base",
        display_name: "Whisper Base",
        description: "优点：体积最小、速度最快的 Whisper 模型，适合日常轻量使用\n缺点：准确率在 Whisper 系列中最低，中文识别易出错，不适合专业场景",
        backend: BackendKind::Whisper,
        total_size_bytes: 148_000_000,
        size_display: "142 MB",
        files: WHISPER_BASE_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
    ModelInfo {
        id: "whisper-small",
        display_name: "Whisper Small",
        description: "优点：精度比 Base 显著提升，支持 99 种语言，中英文均衡，性价比高\n缺点：体积比 Base 大 3 倍，推理速度较慢，不如专用模型精准",
        backend: BackendKind::Whisper,
        total_size_bytes: 488_636_416,
        size_display: "466 MB",
        files: WHISPER_SMALL_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
    ModelInfo {
        id: "whisper-medium",
        display_name: "Whisper Medium",
        description: "优点：多语言转录精度优秀，中英混合语音表现好\n缺点：体积 1.4 GB 较大，推理速度偏慢，Mac 上可能有明显延迟",
        backend: BackendKind::Whisper,
        total_size_bytes: 1_533_774_848,
        size_display: "1.4 GB",
        files: WHISPER_MEDIUM_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
    ModelInfo {
        id: "whisper-large-v3",
        display_name: "Whisper Large v3",
        description: "优点：Whisper 最大最精准模型，支持 99 种语言，准确率最高\n缺点：体积 2.9 GB，推理速度最慢，实时语音输入可能有较长等待",
        backend: BackendKind::Whisper,
        total_size_bytes: 3_094_623_232,
        size_display: "2.9 GB",
        files: WHISPER_LARGE_V3_FILES,
        best_for_languages: &["auto", "fr", "de", "es", "ru"],
        recommendation_reason: "该语言下 Whisper Large v3 准确率最高",
    },
    ModelInfo {
        id: "whisper-large-v3-turbo",
        display_name: "Whisper Large v3 Turbo",
        description: "优点：Large v3 的加速版，速度提升数倍但精度接近原版，英语表现优秀\n缺点：体积仍有 1.5 GB，小语种精度不如完整 Large v3",
        backend: BackendKind::Whisper,
        total_size_bytes: 1_620_000_000,
        size_display: "1.5 GB",
        files: WHISPER_LARGE_V3_TURBO_FILES,
        best_for_languages: &["en"],
        recommendation_reason: "英文识别精度最高，速度比 Large v3 快数倍",
    },
    ModelInfo {
        id: "whisper-large-v3-turbo-q5",
        display_name: "Whisper Large v3 Turbo (Q5)",
        description: "优点：Large v3 Turbo 的 5-bit 量化版，体积仅为原版 1/3，精度损失极小\n缺点：量化可能导致极少数场景下转录异常，整体仍是性价比之选",
        backend: BackendKind::Whisper,
        total_size_bytes: 574_000_000,
        size_display: "547 MB",
        files: WHISPER_LARGE_V3_TURBO_Q5_FILES,
        best_for_languages: &[],
        recommendation_reason: "",
    },
    // --- SenseVoice family ---
    ModelInfo {
        id: "sensevoice-small",
        display_name: "SenseVoice Small",
        description: "优点：阿里出品，中日韩语音识别最佳，推理极快（RTF<0.1），自动添加标点\n缺点：英文及其他欧洲语言精度不如 Whisper，仅支持中日韩英粤 5 种语言",
        backend: BackendKind::SenseVoice,
        total_size_bytes: 239_316_000,
        size_display: "228 MB",
        files: SENSEVOICE_SMALL_FILES,
        best_for_languages: &["zh", "ja", "ko"],
        recommendation_reason: "中日韩语识别精度远超 Whisper，推理速度快且自动添加标点",
    },
    // --- Paraformer family ---
    ModelInfo {
        id: "paraformer-zh",
        display_name: "Paraformer 中文",
        description: "优点：阿里出品，非自回归架构推理极快（RTF<0.07），中文识别精度高\n缺点：仅支持中文，不适合英文或其他语言用户",
        backend: BackendKind::Paraformer,
        total_size_bytes: 227_405_559,
        size_display: "217 MB",
        files: PARAFORMER_ZH_FILES,
        best_for_languages: &["zh"],
        recommendation_reason: "中文专用，推理速度极快（RTF<0.07），精度高",
    },
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
    // --- FireRedASR family ---
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
    // --- Moonshine family ---
    ModelInfo {
        id: "moonshine-base-en",
        display_name: "Moonshine Base (EN)",
        description: "优点：专为实时语音设计，推理速度约为 Whisper 的 5 倍（RTF<0.05），英文精度好\n缺点：仅支持英文，不适合中文或其他语言用户",
        backend: BackendKind::Moonshine,
        total_size_bytes: 286_929_760,
        size_display: "274 MB",
        files: MOONSHINE_BASE_EN_FILES,
        best_for_languages: &["en"],
        recommendation_reason: "英文专用，推理速度约为 Whisper 的 5 倍",
    },
    // --- Zipformer CTC family ---
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
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Look up a model by its unique ID.
pub fn get_model(id: &str) -> Option<&'static ModelInfo> {
    ALL_MODELS.iter().find(|m| m.id == id)
}

/// Return all recommended models for a given language code.
pub fn recommended_models_for_language(language: &str) -> Vec<&'static ModelInfo> {
    ALL_MODELS
        .iter()
        .filter(|m| m.best_for_languages.contains(&language))
        .collect()
}

/// Check whether the current model is already one of the recommended ones for the language.
/// Returns `Some(vec of (id, name, reason))` if a switch is suggested, `None` if current model
/// is already in the recommended set (or no recommendations exist for this language).
pub fn suggest_model_switch(
    current_model: &str,
    language: &str,
) -> Option<Vec<(&'static str, &'static str, &'static str)>> {
    let recommended = recommended_models_for_language(language);
    if recommended.is_empty() {
        return None;
    }
    // If the user already uses one of the recommended models, no suggestion needed.
    if recommended.iter().any(|m| m.id == current_model) {
        return None;
    }
    Some(
        recommended
            .into_iter()
            .map(|m| (m.id, m.display_name, m.recommendation_reason))
            .collect(),
    )
}

const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";

/// Replace the default HuggingFace endpoint in a model file URL with a custom one.
/// If `hf_endpoint` is empty or equals the default, the original URL is returned unchanged.
pub fn resolve_url(original_url: &str, hf_endpoint: &str) -> String {
    if hf_endpoint.is_empty() || hf_endpoint == DEFAULT_HF_ENDPOINT {
        return original_url.to_string();
    }
    let endpoint = hf_endpoint.trim_end_matches('/');
    original_url.replacen(DEFAULT_HF_ENDPOINT, endpoint, 1)
}
