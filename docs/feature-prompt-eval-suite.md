# 提示词评估套件 + 默认提示词重优化

## 状态：设计中 🛠

## 需求描述

为 `src-tauri/src/llm/client.rs` 中的"语音转文字后处理"提示词建立一套端到端评估机制，并基于评估结果迭代优化默认提示词，验收线为 **200 条用例 100% 通过**。

三个核心场景：

1. **中英混杂** — 中文中嵌入英文术语 / 缩写 / 代码 token，含音译还原（瑞嗯特 → React 等）
2. **口吃 / 语音重复** — 语气词、重复词、改口、字母拼读补充、数量改口连锁
3. **文本结构化需求** — `text_structuring=true` 时按顺序词产出编号列表 + 总分一致 + 单点禁编号 + 子项 (a)(b)(c) + 语境感知；`text_structuring=false` 时一律纯文本

跨中英双语覆盖。

## 非目标

- 不替换/重构现有 225 个 cargo 单元测试（它们测 prompt 字符串生成，是另一层）
- 不改 LLM 主模型（仍 `gpt-4o-mini`）、不改 `OptimizeOptions` 结构、不引入新前端 UI
- 不在 CI 中默认运行评估（费 API、需 key、跨外网）

## 架构概览

```
┌──────────────────────────────────────────────────────────────┐
│  数据集（静态 JSON，200 条）                                  │
│  src-tauri/tests/data/prompt_eval_cases.json                  │
└──────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  Runner（Rust integration test，#[ignore] 默认不跑）          │
│  src-tauri/tests/prompt_eval.rs                               │
│  ─ 加载 config.toml 拿 api_key/base_url                       │
│  ─ 并发（8 并发）调 LlmClient.optimize_text                   │
│  ─ 每条用例跑启发式（must_contain / regex / form / 等）       │
│  ─ 输出 JSON 报告 → tmp/prompt_eval_report.json               │
└──────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  Codex CLI 判分（仅主观项）                                   │
│  Claude 读报告 → 把 needs_judge=true 的 case 批量喂给 codex   │
│  → 得到 yes/no + 理由 → 折回报告，决定 prompt 改动方向        │
└──────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  迭代收敛                                                     │
│  Claude 修改 client.rs（zh_body / en_body / 结构化模块）      │
│  → 重跑 → 直至 200/200 pass                                   │
└──────────────────────────────────────────────────────────────┘
```

## 数据集设计

文件：`src-tauri/tests/data/prompt_eval_cases.json`

### 用例 schema

```jsonc
{
  "id": "mixed-zh-001",                  // 唯一 id：{scenario}-{lang}-{NNN}
  "scenario": "mixed",                   // mixed | stutter | structure
  "language": "zh",                      // zh | en —— 决定送给 LLM 的 language 参数
  "text_structuring": false,             // 是否启用结构化模块
  "input": "我们用瑞嗯特做前端，然后通过诶辟爱调后端",
  "checks": {
    "must_contain": ["React", "API"],    // 子串都必须出现
    "must_not_contain": ["瑞嗯特", "诶辟爱"],
    "must_match_regex": [],              // Rust regex 语法
    "no_markdown": true,                 // 禁止 markdown（# / * / - / ``` / 行首 N. 列表）
    "form": "plain",                     // plain | numbered_list | auto
    "min_chars": 10,                     // sanity 下界（避免空输出过判）
    "max_chars": 200                     // sanity 上界（避免扩写过判）
  },
  "judge_rubric": null                   // string 或 null —— 非 null 才走 LLM-judge
}
```

`form` 含义：
- `plain`：禁含 `\n1. ` / `\n- ` / 多个独立段落标号
- `numbered_list`：必须含 `\n1. ` 和 `\n2. `（≥2 项）；首行可以是总起句
- `auto`：不检查列表形态（用于结构化场景里的"应当输出 plain"边缘 case，由 must_not_contain 兜底）

### 200 条分布

| 场景 | 中文 | 英文 | 小计 |
|---|---|---|---|
| (a) mixed 中英混杂 | 47 | 20 | 67 |
| (b) stutter 口吃/重复 | 47 | 20 | 67 |
| (c) structure 结构化 | 46 | 20 | 66 |
| **合计** | 140 | 60 | **200** |

structure 场景内部再分：
- `text_structuring=true` + 多项顺序词 → 应输出 numbered_list（约 30 条）
- `text_structuring=true` + 单项 → 应输出 plain（约 12 条，验"单点禁编号"）
- `text_structuring=true` + 非正式吐槽 → 应输出 plain（约 12 条，验"语境感知"）
- `text_structuring=false` + 顺序词触发 → 应输出 plain（约 12 条，验 toggle off 优先）

### Rubric 何时使用

- 启发式可机械化覆盖的（含/不含某词、是/否 markdown、是/否列表）→ 不写 rubric
- 主观判断（"是否保留原意"、"是否正确合并改口"、"是否未扩写"）→ 写 rubric，给 Codex 判
- 估计 ~80 / 200 用例需要 rubric（主要在 stutter 场景的"原意保留"判断）

### Pass 定义（合一）

一条用例 **pass** 的充要条件：

```
heuristic_pass == true
  AND (judge_rubric == null OR judge_result == "pass")
```

含义：

- `judge_rubric == null` 的用例：完全信任启发式
- 有 rubric 的用例：必须启发式 + judge 都通过
- 任一失败即 fail，记入待改清单

## Runner 设计

文件：`src-tauri/tests/prompt_eval.rs`

```rust
#[tokio::test]
#[ignore = "calls real OpenAI API; run manually with --include-ignored"]
async fn full_prompt_eval() -> anyhow::Result<()> {
    let config = load_app_config()?;
    let client = build_client(&config)?;
    let cases = load_cases("tests/data/prompt_eval_cases.json")?;

    let results = run_concurrent(&client, &cases, /* concurrency = */ 8).await?;
    let report = aggregate(&results, &cases);

    write_json("tmp/prompt_eval_report.json", &report)?;
    print_summary(&report);

    let unresolved = report.iter().filter(|r| r.heuristic_failed || r.needs_judge).count();
    eprintln!("⚠ {} / {} cases need attention (judge or fixed)", unresolved, cases.len());
    // 故意不 panic — 失败用例由 Codex judge 后再决定是 prompt 错还是 case 错。
    // Phase 末尾再加一个 strict_full_eval 测试，要求 100% pass 才通过。
    Ok(())
}
```

关键决策：

- **温度 0**：评估调用强制 `temperature=0` 以最大化可复现性。`LlmClient` 新增 `optimize_text_with_temperature(...)` 或在 `OptimizeOptions` 上加 `temperature: Option<f32>` 字段（生产路径传 None，行为不变）。
- **无 history / vocabulary / user_tags / source_app**：评估专注于 prompt 本身，不混入这些上下文（除非用例显式声明）。
- **失败重试**：API 错误重试 1 次（指数退避 2s），仍失败标记 `api_error: true`，不计入 prompt 失败。
- **并发限流**：`tokio::sync::Semaphore` 限 8 并发，避免触发 OpenAI rate limit。
- **API key 来源**：调 `crate::config::load()` 从 `~/Library/Application Support/com.input0.app/config.toml` 读取，缺失则 `eprintln!` 提示后跳过（test ignored 时不 fail）。

## 报告格式

`tmp/prompt_eval_report.json`：

```jsonc
{
  "ran_at": "2026-05-10T12:34:56Z",
  "model": "gpt-4o-mini",
  "total": 200,
  "heuristic_pass": 175,
  "needs_judge": 80,
  "api_errors": 0,
  "by_scenario": {
    "mixed": { "pass": 60, "fail": 7 },
    "stutter": { "pass": 55, "fail": 12 },
    "structure": { "pass": 60, "fail": 6 }
  },
  "cases": [
    {
      "id": "mixed-zh-001",
      "input": "...",
      "output": "...",
      "heuristic": { "pass": true, "failed_checks": [] },
      "needs_judge": false,
      "judge_rubric": null,
      "judge_result": null,
      "judge_reason": null
    },
    {
      "id": "stutter-zh-014",
      "input": "...",
      "output": "...",
      "heuristic": { "pass": false, "failed_checks": ["must_not_contain: '呃'"] },
      "needs_judge": false,
      "judge_rubric": null,
      "judge_result": null,
      "judge_reason": null
    }
    // ...
  ]
}
```

控制台 summary（人类可读）：

```
=== Prompt Eval — 2026-05-10 12:34:56 ===
Total cases: 200
Heuristic pass: 175 / 200 (87.5%)
Needs Codex judge: 80 cases

By scenario:
  mixed       60 / 67  (89.6%)
  stutter     55 / 67  (82.1%)
  structure   60 / 66  (90.9%)

By language:
  zh   125 / 140  (89.3%)
  en    50 /  60  (83.3%)

Failed cases (heuristic):
  - stutter-zh-014: must_not_contain '呃' failed
    output: "呃我觉得吧..."
  - mixed-en-007: must_contain 'JavaScript' failed
    ...

⚠ Run Codex judge on 80 cases needing rubric review.
```

## Codex 判分流程（Claude 主控）

每轮 cargo test 跑完后，Claude 执行：

1. 解析 `tmp/prompt_eval_report.json`，提取 `needs_judge=true` 的 cases（含 rubric / input / output）
2. 按 batch（每批 10 case）构造 codex 输入：

```
你是 STT 后处理输出评估器。下面 10 个用例每条包含：原始转录文本（input）、模型输出（output）、判分标准（rubric）。
对每条输出 JSON：{"id": "...", "pass": true/false, "reason": "..."}
不要解释、不要总结，仅输出 JSON 数组。

[
  { "id": "stutter-zh-001", "input": "...", "output": "...", "rubric": "..." },
  ...
]
```

3. 通过 Bash 调 `codex exec --model gpt-5 --json '<prompt>'`（exact 调用方式以 codex CLI 的实际 flag 为准，写实现时再确认）
4. 解析返回，折回报告，得到完整的 200 条 pass/fail 结果
5. 失败聚类 → 决定下一轮 prompt 改动

## 迭代收敛策略

每轮：
1. 跑 200 条
2. 启发式失败 + judge 失败合并成"待改清单"
3. 按失败模式分组（例：5 条都是"未还原 React"、3 条都是"输出了 markdown"）
4. 修改 `client.rs` 中：
   - `zh_body(language)` / `en_body(language)` — 主规则
   - `zh_structuring_module()` / `en_structuring_module()` — 结构化规则
5. 不改 boilerplate 处理、信封、`is_custom_prompt_active`、legacy migration（这些是基础设施，不属于 prompt 调优范围）
6. 重跑

预算：5–10 轮收敛。每轮 200 主调 ≈ $0.03（gpt-4o-mini），Codex judge 走 Codex 订阅（不计 OpenAI 配额）。总计 < $0.5。

## 改动文件

| 文件 | 改动 |
|---|---|
| `src-tauri/tests/data/prompt_eval_cases.json` | 新建，200 条用例 |
| `src-tauri/tests/prompt_eval.rs` | 新建，runner + heuristic + report |
| `src-tauri/tests/common/mod.rs` | 可能新建，共享 helpers（heuristic check / report struct） |
| `src-tauri/src/llm/client.rs` | 1) 加 `temperature` 支持（OptimizeOptions 或新 helper）；2) 迭代改 `zh_body` / `en_body` / `zh_structuring_module` / `en_structuring_module` |
| `src-tauri/src/llm/mod.rs` | 视需要 re-export `pub` 接口给 integration test 用 |
| `src-tauri/Cargo.toml` | dev-dependencies 加 `anyhow`、`tokio` features 确认、可能 `regex` 已有 |
| `docs/feature-prompt-eval-suite.md` | 本文档 |
| `CLAUDE.md` | Documentation Map 加一行 |

不改：

- `pipeline.rs` / `commands/llm.rs` / 其他生产路径
- 现有 `src-tauri/src/llm/tests.rs`（225 单元测试不动）
- 任何前端代码

## 验收标准

1. `cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture` 跑出 200 / 200 pass（heuristic + Codex judge 联合）
2. `cargo test --lib` 仍 225 passed / 0 failed（没破坏现有单测）
3. `pnpm build` 仍通过（前端无影响）
4. `client.rs` 中 `zh_body` / `en_body` / 结构化模块已基于评估结果更新
5. `tmp/prompt_eval_report.json` 留存最后一轮的成功报告作为证据

## 风险与缓解

| 风险 | 缓解 |
|---|---|
| LLM 非确定性导致 100% pass 不可达 | `temperature=0` + 用例严格 well-defined（避免主观要求） |
| Codex judge 自身误判 | rubric 写得越客观越好；可疑 case 人工复审；失败 ≥3 次重判 |
| 用例本身写错（误判好输出为 fail） | 第 1 轮跑完后审计低 pass 率的用例集合，修正用例而非 prompt |
| OpenAI rate limit / 网络抖动 | 8 并发 + 重试 1 次 + 失败不计入 prompt 错误 |
| 改 prompt 引入回归 | 每轮跑完都对比上轮 pass 率，下降的 case 优先调查 |
| 既存自定义 prompt 用户的 legacy 模板 | `legacy_v3_default_template` 已锁定旧模板字节快照；本次新 default 触发 `is_legacy_default_template` 路径，影响为零 |

## 后续（不在本次范围）

- 把评估接入 release pre-flight checklist
- 评估覆盖更多真实用户语音样本（脱敏后）
- LLM-judge 模型升级到 GPT-5.4 后再校准 rubric
- 探索把"温度 0"也用到生产（可能进一步降低输出抖动）
