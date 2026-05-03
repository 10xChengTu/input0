# 输出语言支持繁体中文

## 状态：已完成 ✅

## 目标

在「语音设置」语言下拉中，把当前的「中文 (Chinese)」拆分为两个独立选项：

- **简体中文 (Chinese Simplified)** — 内部 code `zh-CN`
- **繁體中文 (Chinese Traditional)** — 内部 code `zh-TW`

用户显式选择某一变体后，整条管线（Whisper initial_prompt、LLM 后处理、模型推荐）都按该目标变体输出。

## 非目标

- 不实现「自动判定原始语料是简还是繁」（语言=auto 时维持现状，由 STT 与 LLM 联合输出，不强转）
- 不引入 OpenCC 等额外简繁转换库；繁体输出能力来自 Whisper initial_prompt 引导 + LLM 指令，不做后处理转换
- 不改 Whisper / sherpa-onnx 上游 API 调用方式（这些 API 不区分简繁；映射在适配层完成）

## 背景：当前状态

| 层 | 现状 |
|---|---|
| 配置 | `AppConfig.language: String`，下拉值有 `auto`/`en`/`zh`/`ja`/`ko`/`es`/`fr`/`de` |
| Whisper | `initial_prompt_for_language("zh") → "以下是普通话的句子。"` 偏向简体输出（见 `feature-zh-initial-prompt.md`） |
| SenseVoice | `map_language_for_sensevoice("zh") → "zh"`，输出由模型决定（默认偏简体） |
| Paraformer / FireRedASR / Zipformer-CTC | 忽略 language 参数，模型固定输出简体 |
| Moonshine | 仅英文，不参与 |
| LLM Prompt | `build_system_prompt` 在 `language == "zh"` 时进入 zh 分支；规则 2 明确 "保留中文变体（简体/繁体），不互相转换" |
| 模型推荐 | `recommended_models_for_language("zh")` 推荐中文友好的 STT 模型 |

## 设计

### 1. 语言 code 体系

引入两个新 code 替代旧的 `zh`：

| 旧 code | 新 code | 显示文本 |
|---|---|---|
| `zh` | `zh-CN` | 简体中文 (Chinese Simplified) |
| —    | `zh-TW` | 繁體中文 (Chinese Traditional) |

其他语言 code（`auto` / `en` / `ja` / `ko` / `es` / `fr` / `de`）不变。

### 2. 各层行为

#### 配置层

`AppConfig.language` 仍为 `String`，新合法值为 `zh-CN` 与 `zh-TW`。

**向后兼容**：在 `config::load_from_dir` 中，读到 `language == "zh"` 时归一化为 `"zh-CN"` 并立即写回（best-effort），与现有 `custom_prompt` 迁移走同一段逻辑，避免下游分散判断。

#### STT 层

新增辅助函数 `crate::stt::language_to_stt_lang(code: &str) -> &str`，把 UI 层 code 折叠成上游 STT API 能识别的值：

```
zh-CN | zh-TW → "zh"
其他   →  原值
```

**Whisper（`whisper/transcriber.rs` + `stt/whisper_backend.rs`）**：
- 调 `params.set_language(...)` 时使用折叠后的 `"zh"`
- `initial_prompt_for_language` 扩展：
  - `zh-CN` → `Some("以下是普通话的句子。")`（保持现状）
  - `zh-TW` → `Some("以下是國語的句子。")`（新增；用繁体字符引导，参考 `feature-zh-initial-prompt.md` 的引导原理）
  - 其他 → `None`
- `whisper/transcriber.rs` 与 `stt/whisper_backend.rs` 内有重复的同名函数，两处同步修改

**SenseVoice（`stt/sensevoice_backend.rs`）**：
- `map_language_for_sensevoice` 把 `zh-CN` 与 `zh-TW` 都映射为 `"zh"`

**其他 sherpa-onnx 后端（Paraformer / FireRedASR / Zipformer-CTC）**：
- 当前忽略 language 参数，无需改动；繁体输出靠 LLM 转换（见已知限制）

#### LLM 层（`llm/client.rs`）

入口 `build_system_prompt` 当前用 `language == "zh"` 二分。改为：

```
let zh_branch = matches!(language, "zh-CN" | "zh-TW");
```

`zh_branch` 为 true 时进入中文 prompt 路径，否则走英文路径。

**规则改写（zh_body 与 en_body 的规则 2）**：

当前规则 2 含 "保留中文变体（简体/繁体），不互相转换"。改为按 `language` 分三档：

| language | 规则 2 中关于变体的子句 |
|---|---|
| `zh-CN` | "请输出简体中文；如原文包含繁体字符，转换为对应的简体。" |
| `zh-TW` | "請輸出繁體中文；如原文包含簡體字符，轉換為對應的繁體。" |
| `auto` 或其他 | 维持现状："保留中文变体（简体/繁体），不互相转换。" |

实现：把规则 2 的尾巴拆为参数（`variant_directive: &str`），由 `build_system_prompt` 根据 `language` 注入。`zh_body()` 与 `en_body()` 改成接受这一参数的函数。

**默认模板与自定义 prompt（`build_default_template` / `is_custom_prompt_active`）**：

- `build_default_template(language)` 已是 language-aware；带上新的 variant_directive 后，`zh-CN` 与 `zh-TW` 自然会渲染出不同模板。
- `is_custom_prompt_active` 当前对比 `build_default_template(current_language)`。用户切换语言时，旧默认模板与新默认模板不再字节相等，会被误判为"自定义"。
- 解决：在 `is_custom_prompt_active` 中，把 `[zh-CN, zh-TW]` 视为同一族——若用户的 saved prompt 等于该族中**任一**变体的默认模板，仍判为"非自定义"。
- 同样在 `is_legacy_default_template` 中扩展，覆盖两类历史默认：
  - **v3-with-`zh`**：当前版本（带本特性前）的默认模板，由 `build_default_template("zh")` 产生（规则 2 含"保留变体不互相转换"）。新增 `legacy_v3_default_template(language)` 私有函数，只用于识别，且把 `"zh"` 加入识别迭代列表。
  - **v1 / v2**：现有 `legacy_v1_default_template` / `legacy_v2_default_template` 已覆盖；只需把 `"zh-CN"` / `"zh-TW"` 加入迭代以容错（这两个 code 在 v1/v2 时代不存在，本质 no-op，但加上更对称）。

迁移路径合并示例：用户原本 `language = "zh"` + custom_prompt = 当前 v3 默认模板 → 启动后：
1. config 层把 `zh` 改写为 `zh-CN`
2. `is_custom_prompt_active("zh-CN")` 比对 `build_default_template("zh-CN")` 不相等
3. 落入 `is_legacy_default_template`，匹配 `legacy_v3_default_template("zh")` 命中
4. 视为"非自定义"，后续走系统默认（含新的简体目标变体指令）。
   配套 `clear_legacy_default_custom_prompt_on_load` 同步把 `custom_prompt` 清空。

**自定义 prompt 的安全尾巴**：

`feature-custom-prompt.md` 描述的 "安全尾巴" 机制由系统在用户自定义 prompt 后追加固定指令。当用户启用自定义 prompt + 选了 `zh-CN`/`zh-TW` 时，把 variant_directive（同上表）追加到安全尾巴里，避免用户的自定义 prompt 没有这条规则导致输出变体不符合预期。

#### 模型推荐（`models/registry.rs`）

`recommended_models_for_language` 与 `best_for_languages` 字段保持原样（仍用 `"zh"`）。在该函数入口处把 `zh-CN` / `zh-TW` 折叠为 `"zh"` 再做匹配。

#### 前端

**Settings 下拉（`src/components/SettingsPage.tsx`）**：
将原 `<option value="zh">中文 (Chinese)</option>` 改为：

```html
<option value="zh-CN">简体中文 (Chinese Simplified)</option>
<option value="zh-TW">繁體中文 (Chinese Traditional)</option>
```

**Settings store（`src/stores/settings-store.ts`）**：
- 默认值仍为 `"auto"`
- 在 `loadConfig` 路径里兜底：若从后端拿到 `language === "zh"`，归一化为 `"zh-CN"`（防御性，正常路径已由后端 migration 处理）
- `checkModelRecommendation(language)` 调用前不需改，因后端 `get_model_recommendation` 已折叠

**HistoryPage（`src/components/HistoryPage.tsx`）**：
当前 `locale === "zh" ? "zh-CN" : "en-US"`。改为：

```
locale === "zh-TW" ? "zh-TW"
: (locale === "zh-CN" || locale === "zh") ? "zh-CN"
: "en-US"
```

兼容仍可能在历史记录里残留的 `"zh"` 字符串。

### 3. 文件改动清单

**Rust（src-tauri/src/）**：
| 文件 | 改动 |
|---|---|
| `config/mod.rs` | 在 `load_from_dir` 中归一化 `zh → zh-CN`，写回磁盘 |
| `config/tests.rs` | 新增迁移测试 |
| `whisper/transcriber.rs` | `initial_prompt_for_language` 支持 `zh-CN`/`zh-TW`；调底层 API 时折叠 |
| `whisper/tests.rs` | 增加 `zh-CN` / `zh-TW` 的 prompt 断言 |
| `stt/whisper_backend.rs` | 同上（两处需保持一致） |
| `stt/sensevoice_backend.rs` | `map_language_for_sensevoice` 折叠 `zh-CN`/`zh-TW` |
| `stt/mod.rs` | 新增公用辅助 `language_to_stt_lang`（可选；若分散于各 backend 也可接受） |
| `llm/client.rs` | `build_system_prompt` 改用 `zh_branch` 判定；`zh_body` / `en_body` 接收 `variant_directive` 参数；`is_custom_prompt_active` / `is_legacy_default_template` 扩展 zh 族识别；自定义 prompt 安全尾巴注入 variant_directive |
| `llm/tests.rs` | 增加 `zh-CN` / `zh-TW` 的 prompt 内容断言（包含目标变体指令、不保留"不互相转换"规则） |
| `models/registry.rs` | `recommended_models_for_language` 入口折叠 `zh-CN`/`zh-TW` 到 `zh` |

**TypeScript（src/）**：
| 文件 | 改动 |
|---|---|
| `components/SettingsPage.tsx` | 下拉 option 拆分；i18n 文案（若有 `t.settings.languageLabel` 之类的字典需更新） |
| `stores/settings-store.ts` | `loadConfig` 防御性归一化 |

**文档**：
| 文件 | 改动 |
|---|---|
| `CLAUDE.md` | Documentation Map 加入本文件，更新 `feature-zh-initial-prompt.md` 的"最后校验"日期（其内容仍正确，但需指向本文件作为后续扩展） |
| `docs/feature-zh-initial-prompt.md` | 末尾「扩展说明」节里把"如需支持繁体中文输出"改为"已扩展"，链接本文件 |
| `docs/landing-page-brief.md` | 功能清单里新增"支持繁体中文输出" |
| `docs/feature-custom-prompt.md` | 安全尾巴章节补充：当语言为 zh-CN/zh-TW 时追加目标变体指令 |

### 4. 已知限制

1. **非 Whisper 的中文 STT 模型本身没有繁体输出能力**：Paraformer / FireRedASR / Zipformer-CTC / SenseVoice 都会输出简体；繁体输出依赖 LLM 阶段转换。**若用户禁用 LLM 优化（API key 为空 / 调用失败），且选了 `zh-TW`，输出会回落为简体。** 这是当前架构的固有限制，不在本次 scope 内处理；在 `feature-zh-initial-prompt.md` 与本文件的「已知限制」里写明即可。
2. **Whisper initial_prompt 引导有概率漏字**：base/small 模型偏置偶尔失败，与简体场景同理。文档保留"使用 medium/large 更稳定"的提示。
3. **历史记录的日期格式**：HistoryPage 使用 UI 翻译语言（`Locale = "zh" | "en"`）驱动日期格式化，不读用户的转录目标语言。所以选择 `zh-CN` / `zh-TW` 不影响历史记录的日期显示。这是有意设计——UI 语言和转录语言是两个独立维度。

## 测试计划

**Rust 单元测试（`cargo test --lib`）**：
- `whisper::tests` — `zh-CN` 返回简体 prompt、`zh-TW` 返回繁体 prompt、其他语言返回 `None`、`zh` 仍返回简体（向后兼容期）
- `config::tests` — 旧 `language = "zh"` 加载后归一化为 `"zh-CN"` 并写回
- `llm::tests` — `zh-CN` prompt 含「请输出简体中文」、`zh-TW` prompt 含「請輸出繁體中文」；两者均**不**含「保留变体不互相转换」；`auto` 仍含原规则
- `llm::tests` — `is_custom_prompt_active`：用户保存的是 `zh-CN` 默认模板、当前 language 切到 `zh-TW`，仍判为 false
- `llm::tests` — `is_legacy_default_template`：v3-with-`zh` 默认模板能被识别（迁移用户的 unmodified custom_prompt 不会被误判为自定义）
- `models::registry` 间接测试：`recommended_models_for_language("zh-TW")` 返回与 `"zh"` 一致的列表

**手工验证**：
1. 全新安装 → 选 `zh-TW` → 中文录音 → 确认 Whisper（base 模型够）输出繁体 + LLM 输出繁体
2. 同样录音切到 `zh-CN` → 确认输出简体
3. 老用户配置（手工把 `config.toml` 写入 `language = "zh"`）→ 启动应用 → 确认 UI 显示「简体中文」、配置文件被改写为 `zh-CN`
4. 选 `zh-TW` + 启用 Paraformer → 录音 → 确认 LLM 把模型输出的简体转为繁体
5. 关闭 LLM（清空 API key）+ Paraformer + `zh-TW` → 录音 → 确认输出为简体（限制兜底）+ 无报错
6. 自定义 prompt 启用 + `zh-CN` → `zh-TW` 切换 → 检查 `is_custom_prompt_active` 行为（编辑器未改时不应变成「自定义」）

## 实现顺序建议

1. 配置层迁移 + 测试（基础）
2. STT 层适配 + Whisper 繁体 prompt + 测试
3. LLM 层 prompt 改造 + 自定义 prompt 兼容 + 测试
4. 模型推荐折叠
5. 前端下拉 + HistoryPage locale
6. 文档同步（CLAUDE.md / landing-page-brief / feature-zh-initial-prompt / feature-custom-prompt）
7. 全量手工验证

每一步都可独立提交、不破坏现有功能。

## 参考

- `docs/feature-zh-initial-prompt.md` — 简体 initial_prompt 的实现根因
- `docs/feature-custom-prompt.md` — 自定义 prompt 的安全尾巴机制
- [OpenAI Whisper Discussion #277](https://github.com/openai/whisper/discussions/277) — initial_prompt 引导原理
