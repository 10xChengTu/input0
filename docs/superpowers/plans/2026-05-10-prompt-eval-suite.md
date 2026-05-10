# Prompt Eval Suite + Default Prompt Re-Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust integration test that runs 200 STT-postprocessing prompt eval cases against the real OpenAI API (using the user's existing app config), pair it with a Codex-CLI-driven judge for subjective rubrics, then iterate the `client.rs` default prompts until 200/200 pass.

**Architecture:** Static JSON dataset (`tests/data/prompt_eval_cases.json`) + single integration test (`tests/prompt_eval.rs`) that loads `AppConfig`, fans out 8-concurrent calls to `LlmClient.optimize_text_with_temperature(...)` with `temperature=0`, runs deterministic heuristic checks, and writes a JSON report to `tmp/prompt_eval_report.json`. Cases needing subjective judgment carry a `judge_rubric` string; those are batched out to `codex exec` by the agent driving the iteration loop. Iteration tweaks `zh_body` / `en_body` / `zh_structuring_module` / `en_structuring_module` until convergence.

**Tech Stack:** Rust 1.x stable, tokio, reqwest, serde/serde_json, regex (all already in `src-tauri/Cargo.toml`); Codex CLI 0.130.0 for subjective judging.

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `src-tauri/tests/data/prompt_eval_cases.json` | NEW | 200 eval cases (id / scenario / language / text_structuring / input / checks / judge_rubric) |
| `src-tauri/tests/prompt_eval.rs` | NEW | Single-file integration test: loader + heuristic checker + concurrent runner + report writer |
| `src-tauri/src/llm/client.rs` | MODIFY | Add `optimize_text_with_temperature(...)` (delegated from existing `optimize_text` with `temperature=None`); iterate `zh_body` / `en_body` / `*_structuring_module` based on eval feedback |
| `src-tauri/Cargo.toml` | MODIFY | Add `anyhow = "1"` to `[dev-dependencies]` |
| `docs/feature-prompt-eval-suite.md` | EXISTS | Spec doc, written in brainstorming phase |
| `CLAUDE.md` | MODIFY | Add Documentation Map row + 「最后校验」 date |
| `tmp/prompt_eval_report.json` | RUNTIME ARTIFACT | Per-run report; not committed |

**Why one test file (not split into modules):** Eval suite is self-contained, only one entry point uses the helpers, and Rust integration tests with shared modules require the `tests/common/mod.rs` dance — overkill for ~400 lines of code. Keep it flat.

---

## Task 1: Add `temperature` Override to `LlmClient`

**Files:**
- Modify: `src-tauri/src/llm/client.rs` — add `temperature` field to `ChatRequest`, add public `optimize_text_with_temperature(...)`, refactor `optimize_text` to delegate
- Modify: `src-tauri/src/llm/tests.rs` — add unit test verifying request body includes/omits `temperature`

- [ ] **Step 1: Write failing unit test for request body shape**

Append to `src-tauri/src/llm/tests.rs` (find the existing `mod tests { ... }` block; add inside):

```rust
#[test]
fn test_chat_request_serializes_temperature_when_set() {
    use serde_json::json;
    let req = super::client::ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![],
        temperature: Some(0.0),
    };
    let serialized = serde_json::to_value(&req).unwrap();
    assert_eq!(serialized["temperature"], json!(0.0));
}

#[test]
fn test_chat_request_omits_temperature_when_none() {
    let req = super::client::ChatRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![],
        temperature: None,
    };
    let serialized = serde_json::to_value(&req).unwrap();
    assert!(serialized.get("temperature").is_none(),
        "temperature field should be omitted when None");
}
```

Note: `ChatRequest` is currently private in `client.rs`. The test references `super::client::ChatRequest`. After Step 3 we need to bump it to `pub(crate)`. If the test file's module path differs (check current top-of-file `use super::*;` pattern), adjust the reference accordingly — the existing `pub(crate) struct ChatMessage` in `client.rs:705` shows the convention.

- [ ] **Step 2: Run test to verify failure**

```bash
cd src-tauri && cargo test --lib test_chat_request_serializes_temperature -- --nocapture 2>&1 | tail -20
```

Expected: compile error (no `temperature` field on `ChatRequest`).

- [ ] **Step 3: Modify `ChatRequest` to add optional `temperature`**

Edit `src-tauri/src/llm/client.rs:710-714`:

```rust
#[derive(Serialize)]
pub(crate) struct ChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f32>,
}
```

Both existing `ChatRequest { model, messages }` literals (around `client.rs:820` and `client.rs:864` and `client.rs:927` and `client.rs:1018`) need `temperature: None,` appended:

```rust
let request_body = ChatRequest {
    model: self.model.clone(),
    messages,
    temperature: None,
};
```

- [ ] **Step 4: Run unit tests to verify all pass**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -30
```

Expected: 227 passed (225 existing + 2 new).

- [ ] **Step 5: Add public `optimize_text_with_temperature` method**

Append to `impl LlmClient` block in `src-tauri/src/llm/client.rs` (right before the closing `}` of `impl LlmClient`, after `optimize_text_with_options`):

```rust
/// Variant of `optimize_text` that accepts an explicit temperature override.
/// Used by the prompt eval suite to run with `temperature=0` for reproducibility.
/// Production callers should keep using `optimize_text` (which delegates here
/// with `temperature: None`, preserving the OpenAI default).
pub async fn optimize_text_with_temperature(
    &self,
    raw_text: &str,
    language: &str,
    history: &[HistoryEntry],
    text_structuring: bool,
    vocabulary: &[String],
    source_app: Option<&str>,
    user_tags: &[String],
    temperature: Option<f32>,
) -> Result<String, AppError> {
    let url = format!("{}/chat/completions", self.base_url);

    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: build_system_prompt(language, text_structuring, "", vocabulary, user_tags),
        },
    ];

    if let Some(ctx) = build_context_message(history, source_app) {
        messages.push(ctx);
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: wrap_raw_transcript(raw_text),
    });

    let request_body = ChatRequest {
        model: self.model.clone(),
        messages,
        temperature,
    };

    let response = self
        .http_client
        .post(&url)
        .header("Authorization", format!("Bearer {}", self.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Network error: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("(failed to read body)"));
        return Err(AppError::Llm(extract_api_error(status, &body)));
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| AppError::Llm(format!("Failed to parse response JSON: {}", e)))?;

    let choices = chat_response
        .choices
        .ok_or_else(|| AppError::Llm("Response missing 'choices' field".to_string()))?;

    if choices.is_empty() {
        return Err(AppError::Llm("Response contains empty 'choices' array".to_string()));
    }

    let first_choice = choices
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Llm("Response contains empty 'choices' array".to_string()))?;

    Ok(clean_llm_output(&first_choice.message.content))
}
```

Then refactor existing `optimize_text` (around `client.rs:907`) to delegate:

```rust
pub async fn optimize_text(
    &self,
    raw_text: &str,
    language: &str,
    history: &[HistoryEntry],
    text_structuring: bool,
    vocabulary: &[String],
    source_app: Option<&str>,
    user_tags: &[String],
) -> Result<String, AppError> {
    self.optimize_text_with_temperature(
        raw_text, language, history, text_structuring,
        vocabulary, source_app, user_tags, None,
    ).await
}
```

- [ ] **Step 6: Run all unit tests + frontend build to verify zero regression**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -10
```

Expected: all 227 pass.

```bash
cd /Users/zhenghui/Documents/repos/input0 && pnpm build 2>&1 | tail -5
```

Expected: TypeScript build succeeds (no front-end impact).

- [ ] **Step 7: Commit**

```bash
cd /Users/zhenghui/Documents/repos/input0
git add src-tauri/src/llm/client.rs src-tauri/src/llm/tests.rs
git commit -m "feat(llm): support temperature override for eval reproducibility

Add optimize_text_with_temperature() that accepts an explicit Option<f32>
temperature, used by the prompt eval suite to run gpt-4o-mini at temp=0.
Existing optimize_text() now delegates with None, preserving production
behavior (OpenAI default temperature unchanged).

ChatRequest gains a #[serde(skip_serializing_if)] temperature field so the
on-the-wire request body remains byte-identical to the pre-change shape
when temperature is None.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Add `anyhow` Dev Dependency

**Files:**
- Modify: `src-tauri/Cargo.toml` — add `anyhow = "1"` to `[dev-dependencies]`

- [ ] **Step 1: Add anyhow to dev-dependencies**

Edit `src-tauri/Cargo.toml`. Find the existing `[dev-dependencies]` block:

```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
```

Add a line:

```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
anyhow = "1"
```

- [ ] **Step 2: Verify it resolves**

```bash
cd src-tauri && cargo check --tests 2>&1 | tail -10
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(deps): add anyhow as dev dep for prompt eval test

Used as the error type in the upcoming integration test runner where
multiple unrelated error origins (config parse, JSON parse, reqwest, llm
client) need to compose without bespoke error enum boilerplate.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Define Eval Case Schema + Heuristic Checker

**Files:**
- Create: `src-tauri/tests/prompt_eval.rs` (skeleton with types + checker only; runner added in Task 4)

- [ ] **Step 1: Create file with case schema + heuristic checker + unit tests**

```bash
mkdir -p src-tauri/tests/data
```

Write `src-tauri/tests/prompt_eval.rs`:

```rust
//! Integration test that evaluates the production STT-postprocessing prompts
//! against a 200-case dataset by hitting the real OpenAI API. See
//! `docs/feature-prompt-eval-suite.md` for design rationale.
//!
//! The full eval is `#[ignore]` because it (a) costs real money, (b) needs an
//! `api_key` configured in the app's config.toml, and (c) is non-deterministic
//! enough that we don't want it gating CI. Run with:
//!
//!     cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub scenario: String,         // "mixed" | "stutter" | "structure"
    pub language: String,         // "zh" | "en"
    pub text_structuring: bool,
    pub input: String,
    pub checks: Checks,
    #[serde(default)]
    pub judge_rubric: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Checks {
    #[serde(default)]
    pub must_contain: Vec<String>,
    #[serde(default)]
    pub must_not_contain: Vec<String>,
    #[serde(default)]
    pub must_match_regex: Vec<String>,
    #[serde(default)]
    pub no_markdown: bool,
    #[serde(default = "default_form")]
    pub form: Form,
    #[serde(default)]
    pub min_chars: Option<usize>,
    #[serde(default)]
    pub max_chars: Option<usize>,
}

fn default_form() -> Form { Form::Auto }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    Plain,         // must NOT look like a numbered list
    NumberedList,  // must contain "\n1. " AND "\n2. " (≥2 items)
    Auto,          // no form check
}

#[derive(Debug, Clone, Serialize)]
pub struct HeuristicResult {
    pub pass: bool,
    pub failed_checks: Vec<String>,
}

/// Run all heuristic checks declared in `checks` against `output`. Returns a
/// list of human-readable failure descriptions (empty Vec = all checks passed).
pub fn run_heuristics(output: &str, checks: &Checks) -> HeuristicResult {
    let mut failed = Vec::new();

    for needle in &checks.must_contain {
        if !output.contains(needle.as_str()) {
            failed.push(format!("must_contain '{}' missing", needle));
        }
    }

    for forbidden in &checks.must_not_contain {
        if output.contains(forbidden.as_str()) {
            failed.push(format!("must_not_contain '{}' present", forbidden));
        }
    }

    for pat in &checks.must_match_regex {
        match regex::Regex::new(pat) {
            Ok(re) => {
                if !re.is_match(output) {
                    failed.push(format!("must_match_regex '{}' did not match", pat));
                }
            }
            Err(e) => {
                failed.push(format!("invalid regex '{}': {}", pat, e));
            }
        }
    }

    if checks.no_markdown && contains_markdown(output) {
        failed.push("no_markdown violated (found markdown syntax)".to_string());
    }

    match checks.form {
        Form::Plain => {
            if looks_like_numbered_list(output) {
                failed.push("form=plain but output looks like a numbered list".to_string());
            }
        }
        Form::NumberedList => {
            if !looks_like_numbered_list(output) {
                failed.push("form=numbered_list but output is not a numbered list".to_string());
            }
        }
        Form::Auto => {}
    }

    if let Some(min) = checks.min_chars {
        if output.chars().count() < min {
            failed.push(format!("min_chars={} but output has {} chars", min, output.chars().count()));
        }
    }
    if let Some(max) = checks.max_chars {
        if output.chars().count() > max {
            failed.push(format!("max_chars={} but output has {} chars", max, output.chars().count()));
        }
    }

    HeuristicResult {
        pass: failed.is_empty(),
        failed_checks: failed,
    }
}

/// Detect markdown syntax that the prompt explicitly forbids in plain mode.
/// Numbered lists ("1. foo") are NOT counted here — that's the form check's job.
fn contains_markdown(text: &str) -> bool {
    // Heading
    if text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("# ") || t.starts_with("## ") || t.starts_with("### ")
    }) { return true; }
    // Bullet list
    if text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")
    }) { return true; }
    // Code fence
    if text.contains("```") { return true; }
    // Bold/italic markers around words (rough heuristic)
    if text.contains("**") || text.contains("__") { return true; }
    false
}

/// True iff the output looks like a numbered list per our prompt rules:
/// has at least two of "1. ", "2. ", "3. " at line starts.
fn looks_like_numbered_list(text: &str) -> bool {
    let mut count = 0;
    for marker in ["1. ", "2. ", "3. ", "4. ", "5. "] {
        if text.contains(&format!("\n{}", marker))
            || text.starts_with(marker)
        {
            count += 1;
        }
    }
    count >= 2
}

#[cfg(test)]
mod heuristic_tests {
    use super::*;

    fn checks_with_must_contain(needles: &[&str]) -> Checks {
        Checks {
            must_contain: needles.iter().map(|s| s.to_string()).collect(),
            must_not_contain: vec![],
            must_match_regex: vec![],
            no_markdown: false,
            form: Form::Auto,
            min_chars: None,
            max_chars: None,
        }
    }

    #[test]
    fn must_contain_passes_when_all_present() {
        let c = checks_with_must_contain(&["React", "API"]);
        let r = run_heuristics("we use React with the API layer", &c);
        assert!(r.pass);
        assert!(r.failed_checks.is_empty());
    }

    #[test]
    fn must_contain_fails_with_missing_needle() {
        let c = checks_with_must_contain(&["React", "API"]);
        let r = run_heuristics("we use React only", &c);
        assert!(!r.pass);
        assert_eq!(r.failed_checks.len(), 1);
        assert!(r.failed_checks[0].contains("API"));
    }

    #[test]
    fn must_not_contain_fails_when_forbidden_present() {
        let mut c = checks_with_must_contain(&[]);
        c.must_not_contain = vec!["呃".to_string()];
        let r = run_heuristics("呃我觉得吧", &c);
        assert!(!r.pass);
        assert!(r.failed_checks[0].contains("呃"));
    }

    #[test]
    fn no_markdown_catches_headings_bullets_fences_bold() {
        let mut c = checks_with_must_contain(&[]);
        c.no_markdown = true;
        for bad in ["# heading", "- bullet", "```code", "**bold**"] {
            let r = run_heuristics(bad, &c);
            assert!(!r.pass, "should flag markdown in: {bad}");
        }
    }

    #[test]
    fn no_markdown_allows_clean_prose() {
        let mut c = checks_with_must_contain(&[]);
        c.no_markdown = true;
        let r = run_heuristics("这是一段正常的文本，包含标点。Just text.", &c);
        assert!(r.pass);
    }

    #[test]
    fn form_numbered_list_requires_two_items() {
        let mut c = checks_with_must_contain(&[]);
        c.form = Form::NumberedList;
        let r = run_heuristics("总起句\n1. 第一点\n2. 第二点\n3. 第三点", &c);
        assert!(r.pass);

        let r2 = run_heuristics("just one line, no list", &c);
        assert!(!r2.pass);
    }

    #[test]
    fn form_plain_rejects_numbered_list() {
        let mut c = checks_with_must_contain(&[]);
        c.form = Form::Plain;
        let r = run_heuristics("总起句\n1. 第一点\n2. 第二点", &c);
        assert!(!r.pass);
    }

    #[test]
    fn must_match_regex_works() {
        let mut c = checks_with_must_contain(&[]);
        c.must_match_regex = vec![r"\d+%".to_string()];
        assert!(run_heuristics("增长 15% 用户", &c).pass);
        assert!(!run_heuristics("增长百分之十五", &c).pass);
    }
}
```

- [ ] **Step 2: Run heuristic unit tests to verify they all pass**

```bash
cd src-tauri && cargo test --test prompt_eval heuristic_tests -- --nocapture 2>&1 | tail -20
```

Expected: 8 passed, 0 failed.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/prompt_eval.rs
git commit -m "test(prompt-eval): add eval case schema and heuristic checker

Defines EvalCase / Checks / Form types matching docs/feature-prompt-eval-suite.md
schema, plus a deterministic heuristic checker covering must_contain,
must_not_contain, must_match_regex, no_markdown, form (plain/numbered_list/auto),
and min/max char bounds. 8 inline unit tests cover each check.

No runner yet — that lands in the next task. Dataset and orchestration come
after the runner skeleton.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Build Concurrent Runner + Report Writer

**Files:**
- Modify: `src-tauri/tests/prompt_eval.rs` — add runner, config loader, report types, and the actual `#[ignore] #[tokio::test]` entry point

- [ ] **Step 1: Add runtime types + config loader + runner + main test fn**

Append to `src-tauri/tests/prompt_eval.rs` (after the existing code, before or replacing the `mod heuristic_tests`):

```rust
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub id: String,
    pub scenario: String,
    pub language: String,
    pub text_structuring: bool,
    pub input: String,
    pub output: Option<String>,         // None if API errored
    pub api_error: Option<String>,
    pub heuristic: HeuristicResult,
    pub needs_judge: bool,
    pub judge_rubric: Option<String>,
    /// Filled in by the agent after Codex CLI returns; runner leaves it None.
    pub judge_result: Option<bool>,
    pub judge_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvalReport {
    pub ran_at: String,
    pub model: String,
    pub temperature: f32,
    pub total: usize,
    pub heuristic_pass: usize,
    pub needs_judge: usize,
    pub api_errors: usize,
    pub by_scenario: serde_json::Value,
    pub by_language: serde_json::Value,
    pub cases: Vec<CaseResult>,
}

fn load_cases(path: &Path) -> anyhow::Result<Vec<EvalCase>> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let cases: Vec<EvalCase> = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("failed to parse {} as JSON array of EvalCase: {}", path.display(), e))?;
    if cases.is_empty() {
        anyhow::bail!("{} contained zero cases", path.display());
    }
    // Detect duplicate ids (data integrity)
    let mut ids = std::collections::HashSet::new();
    for c in &cases {
        if !ids.insert(c.id.as_str()) {
            anyhow::bail!("duplicate case id: {}", c.id);
        }
    }
    Ok(cases)
}

/// Runs one case with up to one retry on transient API errors.
async fn run_one(
    client: &input0_lib::llm::client::LlmClient,
    case: &EvalCase,
) -> CaseResult {
    let mut last_err: Option<String> = None;
    for attempt in 0..2 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        match client
            .optimize_text_with_temperature(
                &case.input,
                &case.language,
                &[],                   // history
                case.text_structuring,
                &[],                   // vocabulary
                None,                  // source_app
                &[],                   // user_tags
                Some(0.0),             // temperature
            )
            .await
        {
            Ok(out) => {
                let h = run_heuristics(&out, &case.checks);
                return CaseResult {
                    id: case.id.clone(),
                    scenario: case.scenario.clone(),
                    language: case.language.clone(),
                    text_structuring: case.text_structuring,
                    input: case.input.clone(),
                    output: Some(out),
                    api_error: None,
                    heuristic: h,
                    needs_judge: case.judge_rubric.is_some(),
                    judge_rubric: case.judge_rubric.clone(),
                    judge_result: None,
                    judge_reason: None,
                };
            }
            Err(e) => {
                last_err = Some(format!("{:?}", e));
            }
        }
    }
    CaseResult {
        id: case.id.clone(),
        scenario: case.scenario.clone(),
        language: case.language.clone(),
        text_structuring: case.text_structuring,
        input: case.input.clone(),
        output: None,
        api_error: last_err,
        heuristic: HeuristicResult { pass: false, failed_checks: vec!["api_error".to_string()] },
        needs_judge: case.judge_rubric.is_some(),
        judge_rubric: case.judge_rubric.clone(),
        judge_result: None,
        judge_reason: None,
    }
}

fn aggregate(results: &[CaseResult], model: &str, temperature: f32) -> EvalReport {
    use std::collections::HashMap;
    let total = results.len();
    let heuristic_pass = results.iter().filter(|r| r.heuristic.pass).count();
    let needs_judge = results.iter().filter(|r| r.needs_judge).count();
    let api_errors = results.iter().filter(|r| r.api_error.is_some()).count();

    let mut by_scenario: HashMap<String, (usize, usize)> = HashMap::new();
    let mut by_language: HashMap<String, (usize, usize)> = HashMap::new();
    for r in results {
        let p = r.heuristic.pass;
        let s = by_scenario.entry(r.scenario.clone()).or_insert((0, 0));
        if p { s.0 += 1 } else { s.1 += 1 }
        let l = by_language.entry(r.language.clone()).or_insert((0, 0));
        if p { l.0 += 1 } else { l.1 += 1 }
    }
    let to_json = |m: HashMap<String, (usize, usize)>| -> serde_json::Value {
        let obj: serde_json::Map<String, serde_json::Value> = m
            .into_iter()
            .map(|(k, (pass, fail))| {
                (k, serde_json::json!({"pass": pass, "fail": fail}))
            })
            .collect();
        serde_json::Value::Object(obj)
    };

    EvalReport {
        ran_at: chrono::Utc::now().to_rfc3339(),
        model: model.to_string(),
        temperature,
        total,
        heuristic_pass,
        needs_judge,
        api_errors,
        by_scenario: to_json(by_scenario),
        by_language: to_json(by_language),
        cases: results.to_vec(),
    }
}

fn print_summary(report: &EvalReport) {
    eprintln!("\n=== Prompt Eval — {} ===", report.ran_at);
    eprintln!("Model: {}    Temperature: {}", report.model, report.temperature);
    eprintln!("Total cases: {}", report.total);
    eprintln!("Heuristic pass: {} / {} ({:.1}%)",
        report.heuristic_pass, report.total,
        100.0 * report.heuristic_pass as f64 / report.total as f64);
    eprintln!("Needs Codex judge: {}", report.needs_judge);
    eprintln!("API errors: {}", report.api_errors);
    eprintln!("\nBy scenario: {}", serde_json::to_string_pretty(&report.by_scenario).unwrap());
    eprintln!("By language: {}", serde_json::to_string_pretty(&report.by_language).unwrap());

    let failed: Vec<&CaseResult> = report.cases.iter()
        .filter(|r| !r.heuristic.pass)
        .collect();
    if !failed.is_empty() {
        eprintln!("\n--- Heuristic failures ({}) ---", failed.len());
        for r in failed.iter().take(20) {
            eprintln!("  {}: {}", r.id, r.heuristic.failed_checks.join("; "));
            if let Some(out) = &r.output {
                let preview: String = out.chars().take(80).collect();
                eprintln!("    output: {:?}", preview);
            }
        }
        if failed.len() > 20 {
            eprintln!("  ... and {} more", failed.len() - 20);
        }
    }
}

#[tokio::test]
#[ignore = "calls real OpenAI API; run with: cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture"]
async fn full_prompt_eval() -> anyhow::Result<()> {
    use input0_lib::config;
    use input0_lib::llm::client::LlmClient;

    // Load real app config (uses the same api_key the user has set in Settings).
    let cfg = config::load().map_err(|e| anyhow::anyhow!("failed to load app config: {:?}", e))?;
    if cfg.api_key.trim().is_empty() {
        anyhow::bail!("api_key is empty in {:?}; set it via the app Settings UI first",
            config::config_path());
    }

    let client = LlmClient::new(cfg.api_key.clone(), cfg.api_base_url.clone(), Some(cfg.model.clone()))
        .map_err(|e| anyhow::anyhow!("LlmClient::new failed: {:?}", e))?;

    let cases_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/prompt_eval_cases.json");
    let cases = load_cases(&cases_path)?;
    eprintln!("Loaded {} cases from {}", cases.len(), cases_path.display());

    let semaphore = Arc::new(Semaphore::new(8));
    let client = Arc::new(client);

    let mut handles = Vec::with_capacity(cases.len());
    for case in cases.iter().cloned() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            let _p = permit;
            run_one(&client, &case).await
        }));
    }

    let mut results = Vec::with_capacity(cases.len());
    for h in handles {
        results.push(h.await.unwrap());
    }
    // Re-order to original case order for stable diffs
    let id_to_idx: std::collections::HashMap<&str, usize> = cases.iter().enumerate()
        .map(|(i, c)| (c.id.as_str(), i)).collect();
    results.sort_by_key(|r| id_to_idx.get(r.id.as_str()).copied().unwrap_or(usize::MAX));

    let report = aggregate(&results, client.model(), 0.0);

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let report_dir = manifest_dir.parent().unwrap_or(manifest_dir).join("tmp");
    std::fs::create_dir_all(&report_dir)?;
    let report_path = report_dir.join("prompt_eval_report.json");
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    eprintln!("\nReport written to: {}", report_path.display());

    print_summary(&report);

    // Strict pass: every case must heuristic-pass.
    // Subjective judge results (filled in by Codex out-of-band) live in the
    // report file; the strict assertion happens in a follow-up test that reads
    // the augmented report.
    if report.heuristic_pass < report.total {
        anyhow::bail!(
            "Eval not at 100%: {}/{} heuristic pass; {} cases still need attention. \
             Review the report at {} and either (a) fix the prompt and re-run, \
             or (b) run Codex judge on rubric-flagged cases that may be soft-pass.",
            report.heuristic_pass, report.total,
            report.total - report.heuristic_pass,
            report_path.display(),
        );
    }
    Ok(())
}
```

- [ ] **Step 2: Add `chrono` dev dep (used by `aggregate` for ran_at timestamp)**

Edit `src-tauri/Cargo.toml` `[dev-dependencies]`:

```toml
[dev-dependencies]
wiremock = "0.6"
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
anyhow = "1"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
```

- [ ] **Step 3: Verify the runner compiles**

```bash
cd src-tauri && cargo check --tests 2>&1 | tail -20
```

Expected: `Finished`. If `input0_lib` doesn't resolve, look at `src-tauri/Cargo.toml` for the `[lib]` name:

```bash
grep -A 3 "^\[lib\]\|^name " src-tauri/Cargo.toml | head -10
```

The integration test must use whatever name is declared. Adjust the `use input0_lib::...` lines in the test file accordingly. If there's no `[lib]` section, the lib crate name defaults to the package name with `-` → `_`; check `name =` under `[package]`.

Also: `config::load()` and `config::config_path()` and `config::config_dir()` must be `pub`. Verify by grepping:

```bash
grep -n "^pub fn load\|^pub fn config_path\|^pub fn config_dir" src-tauri/src/config/mod.rs
```

If `load()` is private, change `pub fn load_from_dir` (which already exists) and `pub fn config_dir` accessibility, or add a `pub fn load()` thin wrapper. From the existing code we can see `pub fn config_dir()` is `pub`, but `load()` may be `pub(crate)`. If so, the simplest fix is to mark it `pub fn load()` (it's already a thin wrapper around `load_from_dir(&config_dir()?)`).

If you need to make it pub, add to the runner-creation step a sub-step to flip the visibility:

```bash
# Only if cargo check fails on private `config::load`:
sed -i '' 's/^pub(crate) fn load()/pub fn load()/' src-tauri/src/config/mod.rs
# Or edit manually if the prefix differs.
```

- [ ] **Step 4: Run heuristic unit tests still pass (sanity that runner code didn't break the unit tests in same file)**

```bash
cd src-tauri && cargo test --test prompt_eval heuristic_tests -- --nocapture 2>&1 | tail -10
```

Expected: 8 pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tests/prompt_eval.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
# Also: src-tauri/src/config/mod.rs if visibility was bumped
git commit -m "test(prompt-eval): add concurrent runner + report writer

8-concurrent semaphore-limited runner that hits the production gpt-4o-mini
endpoint via LlmClient.optimize_text_with_temperature(temp=0.0). Each case
runs once with one retry on API error. Heuristic results are aggregated
into an EvalReport that is written to tmp/prompt_eval_report.json (per
the docs/feature-prompt-eval-suite.md schema) and summarized to stderr.

The test is #[ignore]'d by default. Run manually:
  cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture

Strict assertion: bails if heuristic_pass < total. Subjective judge results
are filled in out-of-band by the agent driving the iteration loop via
codex exec, then the strict assertion can be relaxed accordingly.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Smoke-Test Runner with One Hand-Authored Case

Before writing 200 cases, prove the plumbing works end-to-end with a single case. This catches API-key issues, lib name issues, and JSON schema mismatches cheaply.

**Files:**
- Create: `src-tauri/tests/data/prompt_eval_cases.json` (initially with 1 case; will be expanded in Tasks 6-8)

- [ ] **Step 1: Write a 1-case JSON file**

Write `src-tauri/tests/data/prompt_eval_cases.json`:

```json
[
  {
    "id": "smoke-zh-001",
    "scenario": "mixed",
    "language": "zh",
    "text_structuring": false,
    "input": "我们用瑞嗯特做前端，然后通过诶辟爱调后端",
    "checks": {
      "must_contain": ["React", "API"],
      "must_not_contain": ["瑞嗯特", "诶辟爱"],
      "must_match_regex": [],
      "no_markdown": true,
      "form": "plain",
      "min_chars": 5,
      "max_chars": 100
    },
    "judge_rubric": null
  }
]
```

- [ ] **Step 2: Run the eval against the 1-case dataset**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | tail -50
```

Expected: PASS — heuristic_pass = 1 / 1. If it fails:

- API key empty → set it via Settings UI first
- `input0_lib` unresolved → fix `use` path per Task 4 Step 3 fallback
- `config::load` private → bump visibility per Task 4 Step 3 fallback
- API error → check `api_base_url` and connectivity

After success, check `tmp/prompt_eval_report.json` exists and contains the 1 case.

- [ ] **Step 3: Commit the smoke dataset**

```bash
git add src-tauri/tests/data/prompt_eval_cases.json
git commit -m "test(prompt-eval): seed dataset with one smoke case

One mixed-zh case that exercises the full pipeline end-to-end (config
load → LlmClient → heuristic check → report write). Used to validate
plumbing before authoring the full 200-case dataset in subsequent commits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Author 67 Mixed-Language Cases

**Files:**
- Modify: `src-tauri/tests/data/prompt_eval_cases.json` — extend from 1 → 67 cases

**Distribution within mixed:** 47 zh + 20 en. Cover:
- Chinese with embedded English term (e.g., "用 GitHub Copilot")
- Chinese with phonetic transliteration (must restore: 瑞嗯特→React, 诶辟爱→API, 杰森→JSON, 泰普斯克瑞普特→TypeScript, 派森→Python, 高→Go, 道克→Docker, etc.)
- Chinese with code identifier (e.g., "调用 useState 钩子")
- English with Chinese fragment (rare but valid)
- English with abbreviation that should stay (e.g., "the SLA was 99.9%")

- [ ] **Step 1: Replace the file with 67 mixed cases**

Open `src-tauri/tests/data/prompt_eval_cases.json` and replace its content with a JSON array of 67 cases following the schema. Use the smoke case as `mixed-zh-001`. Each case must have:

- `id`: `mixed-zh-001` … `mixed-zh-047`, `mixed-en-001` … `mixed-en-020`
- `scenario`: `"mixed"`
- `language`: `"zh"` for Chinese-primary input, `"en"` for English-primary
- `text_structuring`: `false` (structuring is tested in Task 8)
- `input`: realistic raw STT text
- `checks.must_contain`: terms that MUST appear in the corrected output
- `checks.must_not_contain`: phonetic/wrong forms that MUST NOT remain
- `checks.no_markdown`: `true` (always for mixed scenario)
- `checks.form`: `"plain"`
- `checks.min_chars` / `max_chars`: sensible bounds (≈ input length × [0.5, 1.5])
- `judge_rubric`: `null` for unambiguous cases; non-null for cases where "did the model preserve original meaning" needs subjective check (target: ~5 / 67 cases)

Reference cases (illustrative — fill in 67 following these patterns):

```jsonc
[
  {
    "id": "mixed-zh-001",
    "scenario": "mixed", "language": "zh", "text_structuring": false,
    "input": "我们用瑞嗯特做前端，然后通过诶辟爱调后端",
    "checks": {
      "must_contain": ["React", "API"],
      "must_not_contain": ["瑞嗯特", "诶辟爱"],
      "must_match_regex": [], "no_markdown": true, "form": "plain",
      "min_chars": 10, "max_chars": 60
    },
    "judge_rubric": null
  },
  {
    "id": "mixed-zh-002",
    "scenario": "mixed", "language": "zh", "text_structuring": false,
    "input": "前端用泰普斯克瑞普特写，配合杰森解析",
    "checks": {
      "must_contain": ["TypeScript", "JSON"],
      "must_not_contain": ["泰普斯克瑞普特", "杰森"],
      "must_match_regex": [], "no_markdown": true, "form": "plain",
      "min_chars": 10, "max_chars": 60
    },
    "judge_rubric": null
  },
  {
    "id": "mixed-zh-010",
    "scenario": "mixed", "language": "zh", "text_structuring": false,
    "input": "我用 GitHub Copilot 写代码，速度快多了",
    "checks": {
      "must_contain": ["GitHub Copilot"],
      "must_not_contain": [],
      "must_match_regex": [], "no_markdown": true, "form": "plain",
      "min_chars": 10, "max_chars": 50
    },
    "judge_rubric": "Output must keep 'GitHub Copilot' verbatim and the meaning that the speaker uses it to write code faster. Must not invent details (don't add specific tools or projects)."
  },
  {
    "id": "mixed-en-001",
    "scenario": "mixed", "language": "en", "text_structuring": false,
    "input": "we use react with the api layer and json everywhere",
    "checks": {
      "must_contain": ["React", "API", "JSON"],
      "must_not_contain": ["react ", "api ", "json "],
      "must_match_regex": [], "no_markdown": true, "form": "plain",
      "min_chars": 30, "max_chars": 100
    },
    "judge_rubric": null
  }
  // ... 63 more cases following the same schema, ids mixed-zh-003..047 and mixed-en-002..020
]
```

**Authoring guidance:**

- For zh phonetic cases, draw from common terms: React, API, JSON, TypeScript, JavaScript, Python, Go, Docker, Kubernetes (库伯内特斯), Redis (瑞迪斯), MongoDB, GraphQL, Vue, Angular, Webpack, ESLint, Prettier, npm (恩屁恩), Git, GitHub, GitLab, OAuth, JWT, REST, gRPC.
- For zh embedded-English cases, use natural mixed speech: "把 PR 合到 main 分支", "跑 ci 流水线", "用 docker compose 起服务".
- For en cases, focus on capitalization (lowercase STT → proper case): "github" → "GitHub", "javascript" → "JavaScript".
- Keep `must_not_contain` for the WRONG form only (e.g., the phonetic spelling), not for arbitrary substrings the model might legitimately use.
- `must_not_contain` should NOT include lowercase versions of must_contain terms — it's hard to express "lowercase react in isolation" in substring form. Prefer regex if needed: `r"\breact\b"` (lowercase, word-boundary).

- [ ] **Step 2: Verify the JSON parses + has expected count**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | head -3
```

Expected first line: `Loaded 67 cases from .../prompt_eval_cases.json`. If parse error, fix the JSON syntax.

(The test will likely fail at the heuristic-pass-count assertion — that's expected at this stage. We just want to confirm load + run succeed.)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/data/prompt_eval_cases.json
git commit -m "test(prompt-eval): add 67 mixed-language cases (47 zh + 20 en)

Cover (a) phonetic transliteration restoration (瑞嗯特→React etc.),
(b) zh with embedded English terms, (c) zh with code identifiers,
(d) en with proper capitalization. ~5 cases carry judge_rubric for
subjective 'preserve original meaning' verification.

Part of the 200-case prompt eval dataset; stutter and structure
scenarios land in subsequent commits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Author 67 Stutter / Repetition Cases

**Files:**
- Modify: `src-tauri/tests/data/prompt_eval_cases.json` — extend from 67 → 134 cases

**Distribution:** 47 zh + 20 en. Cover the seven sub-patterns:

1. Filler words (呃/啊/嗯/uh/um) — must be removed
2. Word-level repetition ("我我我", "the the the") — collapse to one
3. Phrase-level repetition ("我觉得吧 我觉得吧") — keep one
4. "不是 A 是 B" / "no wait, X" structure — keep B / X
5. Phonetic-then-spell supplement ("瑞嗯特，就是 R-E-A-C-T") — output "React"
6. Number self-correction ("两千三百，不对，两千五百") — keep "2500"
7. Cascading correction ("说三件事…啊算了说两件" + only 2 follow) — adjust count

- [ ] **Step 1: Append 67 stutter cases (ids stutter-zh-001..047, stutter-en-001..020)**

Edit `src-tauri/tests/data/prompt_eval_cases.json`. Insert these new entries after the existing 67 mixed cases (before the closing `]`). Reference patterns (illustrative — author 67 total):

```jsonc
{
  "id": "stutter-zh-001",
  "scenario": "stutter", "language": "zh", "text_structuring": false,
  "input": "呃 我觉得吧 嗯 这个方案 啊 还行",
  "checks": {
    "must_contain": ["这个方案"],
    "must_not_contain": ["呃", "嗯", "啊"],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 5, "max_chars": 30
  },
  "judge_rubric": "Output should be a clean version of '我觉得这个方案还行' or similar — preserve the speaker's positive evaluation, no filler words."
},
{
  "id": "stutter-zh-002",
  "scenario": "stutter", "language": "zh", "text_structuring": false,
  "input": "我我我我觉得这个不对",
  "checks": {
    "must_contain": ["我觉得这个不对"],
    "must_not_contain": ["我我", "我我我"],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 5, "max_chars": 20
  },
  "judge_rubric": null
},
{
  "id": "stutter-zh-010",
  "scenario": "stutter", "language": "zh", "text_structuring": false,
  "input": "明天九点开会，不对，是十点",
  "checks": {
    "must_contain": ["10"],
    "must_not_contain": ["不对", "九点", "9点"],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 5, "max_chars": 30
  },
  "judge_rubric": "Output must reflect the corrected time (10 o'clock / 10:00) and drop the original mistaken '九点' completely."
},
{
  "id": "stutter-zh-020",
  "scenario": "stutter", "language": "zh", "text_structuring": false,
  "input": "用户增长了百分之十五，不对是百分之二十五",
  "checks": {
    "must_contain": ["25%"],
    "must_not_contain": ["15%", "百分之十五", "百分之二十五", "不对"],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 5, "max_chars": 30
  },
  "judge_rubric": null
},
{
  "id": "stutter-en-001",
  "scenario": "stutter", "language": "en", "text_structuring": false,
  "input": "uh I I I think the the the plan is okay",
  "checks": {
    "must_contain": ["plan"],
    "must_not_contain": ["uh ", "I I", "the the"],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 10, "max_chars": 40
  },
  "judge_rubric": null
}
```

**Authoring guidance:**

- For "must_not_contain" of a leading filler like "uh ", include the trailing space to avoid flagging legitimate contractions
- For self-correction cases, both must_not_contain (the wrong value) and must_contain (the right value) are critical
- Use `judge_rubric` for cases where "did the model preserve the speaker's intent through the correction" needs subjective check — target ~25 / 67 stutter cases get rubrics (this scenario has the most subjectivity)
- For en cases focus on common spoken patterns: "you know", trailing "right?", "I mean...", "scratch that", "no wait"

- [ ] **Step 2: Confirm dataset size jumped to 134**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | head -3
```

Expected: `Loaded 134 cases from ...`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/data/prompt_eval_cases.json
git commit -m "test(prompt-eval): add 67 stutter/repetition cases (47 zh + 20 en)

Cover seven sub-patterns: filler removal, word/phrase repetition collapse,
'不是 A 是 B' correction, phonetic-then-spell supplement merging, number
self-correction, and cascading count corrections. ~25 cases carry
judge_rubric since 'preserve speaker intent through correction' is the
most subjective dimension of post-processing.

Dataset now 134 / 200; structure scenario lands next.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Author 66 Structure Cases

**Files:**
- Modify: `src-tauri/tests/data/prompt_eval_cases.json` — extend from 134 → 200 cases

**Distribution:** 46 zh + 20 en, split across four structure-specific sub-buckets:
- 30 cases: `text_structuring=true` + sequence words + ≥2 items → MUST output numbered_list
- 12 cases: `text_structuring=true` + only 1 item → MUST output plain (verifies "no solo numbering")
- 12 cases: `text_structuring=true` + informal rant → MUST output plain (verifies context awareness)
- 12 cases: `text_structuring=false` + sequence words present → MUST output plain (verifies toggle off wins)

- [ ] **Step 1: Append 66 structure cases**

Reference patterns (author 66 total following these):

```jsonc
{
  "id": "structure-zh-001",
  "scenario": "structure", "language": "zh", "text_structuring": true,
  "input": "我说三件事 第一是用户增长上周新增了两千三百个 第二是收入这个月达到了五十万 第三是团队下周要招聘",
  "checks": {
    "must_contain": ["1.", "2.", "3.", "2300", "50"],
    "must_not_contain": ["第一是", "第二是", "第三是", "两千三百"],
    "must_match_regex": [],
    "no_markdown": false,
    "form": "numbered_list",
    "min_chars": 30, "max_chars": 200
  },
  "judge_rubric": "Output must be a numbered list with a brief opening summary mentioning '3' items, then 1./2./3. with concise titled headings. Numbers must be in Arabic digits."
},
{
  "id": "structure-zh-013",
  "scenario": "structure", "language": "zh", "text_structuring": true,
  "input": "我就说一个事 就是明天下午三点开会",
  "checks": {
    "must_contain": ["3"],
    "must_not_contain": ["1."],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 5, "max_chars": 40
  },
  "judge_rubric": "Single point — must NOT use '1.' even though structuring is on. Output should be a natural sentence about a 3pm meeting tomorrow."
},
{
  "id": "structure-zh-025",
  "scenario": "structure", "language": "zh", "text_structuring": true,
  "input": "唉今天好烦啊 客户改需求改了三次 我都快崩溃了",
  "checks": {
    "must_contain": ["客户", "需求"],
    "must_not_contain": ["1."],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 10, "max_chars": 60
  },
  "judge_rubric": "Informal rant — must stay as a natural paragraph, preserve the emotional tone (frustrated). No numbered list even though there's an enumerable element ('three times')."
},
{
  "id": "structure-zh-037",
  "scenario": "structure", "language": "zh", "text_structuring": false,
  "input": "我说三件事 第一是用户增长 第二是收入 第三是团队",
  "checks": {
    "must_contain": ["用户增长", "收入", "团队"],
    "must_not_contain": ["1.", "2.", "3."],
    "must_match_regex": [], "no_markdown": true, "form": "plain",
    "min_chars": 10, "max_chars": 60
  },
  "judge_rubric": "Structuring is OFF — output MUST be a natural sentence even though sequence words are present. No numbered list, no markdown."
},
{
  "id": "structure-en-001",
  "scenario": "structure", "language": "en", "text_structuring": true,
  "input": "three things first user growth two thousand three hundred new this week second revenue hit five hundred k third team is hiring next week",
  "checks": {
    "must_contain": ["1.", "2.", "3.", "2300", "500"],
    "must_not_contain": ["first ", "second ", "third "],
    "must_match_regex": [],
    "no_markdown": false,
    "form": "numbered_list",
    "min_chars": 40, "max_chars": 250
  },
  "judge_rubric": "Numbered list with summary opener mentioning '3' items, Arabic digits for counts, brief titled headings."
}
```

**Authoring guidance:**

- For numbered_list cases, `no_markdown` MUST be false (because numbered lists ARE allowed by the structuring module). Only `form: "numbered_list"` is checked.
- For `text_structuring=false` cases, sequence words in the INPUT must NOT cause numbering in the OUTPUT. This is the "toggle off wins" check.
- For informal-rant cases, judge_rubric is critical — only the model's tone-preservation can be subjectively verified.
- ~30 / 66 structure cases get judge_rubric (numbered_list cases need rubric to verify summary/title/Arabic-digit conversions).

- [ ] **Step 2: Confirm 200 total**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | head -5
```

Expected: `Loaded 200 cases from ...`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/data/prompt_eval_cases.json
git commit -m "test(prompt-eval): add 66 structure cases (46 zh + 20 en); dataset now 200/200

Four sub-buckets: structuring=true with multi-item sequence words
(numbered_list output, ~30 cases), structuring=true with single point
(plain output, ~12), structuring=true with informal rant (plain
output, ~12), structuring=false with sequence words (plain output,
~12). About 30 cases carry judge_rubric for summary-line / title /
digit-conversion verification.

200-case dataset complete; iteration phase begins next.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Iteration Loop — Run Eval, Codex-Judge, Tweak Prompt, Repeat

This task is a **loop** rather than a fixed sequence of steps. Exit when 200/200 pass strictly. Expected: 5–10 iterations.

**Files (each iteration):**
- Modify: `src-tauri/src/llm/client.rs` — `zh_body` (line ~60), `en_body` (line ~124), `zh_structuring_module` (line ~87), `en_structuring_module` (line ~152)
- Read-only: `tmp/prompt_eval_report.json`

- [ ] **Step 1: Run baseline eval**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | tee /tmp/eval-run-baseline.log | tail -80
```

Record the baseline pass rate. Inspect `tmp/prompt_eval_report.json`.

- [ ] **Step 2: For each case with `needs_judge=true`, run Codex judge**

Extract rubric-bearing cases:

```bash
jq '[.cases[] | select(.needs_judge == true) | {id, language, text_structuring, input, output, judge_rubric}]' \
   tmp/prompt_eval_report.json > /tmp/judge_input.json
```

Batch them 10 at a time and feed each batch to Codex CLI. Construct a prompt of the form:

```
You are a strict evaluator for STT post-processing output. For each case below,
decide pass/fail based on the rubric. Output ONLY a JSON array of decisions —
no explanation, no markdown, no extra text.

[
  {"id":"...","input":"...","output":"...","rubric":"..."},
  ...up to 10 per batch...
]

Output schema (one entry per case, in same order):
[{"id":"...","pass":true|false,"reason":"<<10 words"},...]
```

Invoke:

```bash
codex exec "$(cat /tmp/judge_batch.txt)" 2>&1 | tee /tmp/judge_output.txt
```

(Verify exact `codex exec` flags with `codex exec --help` if the above doesn't work — Codex CLI 0.130.0 may use slightly different syntax.)

Parse Codex's JSON output and merge `pass` / `reason` back into each case's `judge_result` / `judge_reason` fields in the report. Then determine the *augmented* pass rate:

```
case_passes = case.heuristic.pass AND (case.judge_rubric is None OR case.judge_result == true)
```

- [ ] **Step 3: Cluster failures by pattern**

For all `case_passes == false` cases, group by:

- Failure type (which `failed_checks` entry, or judge `reason`)
- Scenario (mixed / stutter / structure)
- Language (zh / en)
- Specific phonetic term, structural mistake, etc.

The most common failure pattern is the highest-leverage prompt change. Examples of likely findings (each maps to a different prompt edit):

| Failure pattern | Likely fix in `client.rs` |
|---|---|
| Phonetic terms like 库伯内特斯 / 瑞迪斯 not restored | Extend the inline term list in zh_body rule 2 |
| Number formats not converted (e.g., kept "百分之十五") | Strengthen rule 5; add explicit "even when speech is informal" |
| Filler "呃 " sometimes kept at sentence start | Reinforce rule 1 with "ALWAYS remove" wording |
| Single-item structuring still emits "1." | Strengthen `*_structuring_module` "no solo numbering" wording |
| Informal rant gets numbered | Strengthen "context awareness" sub-rule with concrete examples |
| `text_structuring=false` cases output markdown | Reinforce "# 输出" plain-text default |
| Output begins with "Sure, here's..." / "好的，整理后..." | Add to `LEADING_BOILERPLATE_PREFIXES` (separate fix in `clean_llm_output`) |

- [ ] **Step 4: Apply ONE focused fix at a time**

Pick the highest-impact failure cluster. Edit the relevant prompt text in `src-tauri/src/llm/client.rs`. Do NOT batch multiple unrelated fixes — they obscure which change moved which metric.

Example fix (extending phonetic term list):

```rust
// Before (client.rs zh_body rule 2):
//  "...还原（瑞嗯特→React，诶辟爱→API，杰森→JSON，泰普斯克瑞普特→TypeScript）..."
// After:
//  "...还原（瑞嗯特→React，诶辟爱→API，杰森→JSON，泰普斯克瑞普特→TypeScript，
//   库伯内特斯→Kubernetes，瑞迪斯→Redis，派森→Python，道克→Docker）..."
```

If the fix needs to touch `clean_llm_output` boilerplate stripping (for new opener phrases), add to `LEADING_BOILERPLATE_PREFIXES` (`client.rs:629`).

- [ ] **Step 5: Re-run eval and compare**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | tee /tmp/eval-run-iter-N.log | tail -60
```

Compare the new pass rate with previous. Note any cases that *regressed* (passed before, now fail) — those are higher priority than further forward progress.

- [ ] **Step 6: Loop**

Repeat Steps 2–5 until: every case `case_passes == true` AND `cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture` exits 0 (which only happens when `report.heuristic_pass == report.total`).

If a case repeatedly fails despite reasonable prompt changes:
- Re-read the case → is the test case itself wrong/unreasonable? If yes, fix the case (data, not prompt). Document the change.
- Is it a flake from temperature variance? Re-run that single case 3 times — if it passes ≥2/3, accept and add a comment in the case JSON noting the flake observation.

- [ ] **Step 7: Final commit of prompt changes**

After 200/200 strict pass:

```bash
git add src-tauri/src/llm/client.rs
git commit -m "feat(llm): re-optimize default prompts to pass full 200-case eval

Iterated zh_body / en_body / zh_structuring_module / en_structuring_module
against the new 200-case eval suite (docs/feature-prompt-eval-suite.md)
until every case passed under temperature=0 with gpt-4o-mini.

Concrete changes:
[list the 3-5 most material edits — e.g., 'extended phonetic term list
to cover Kubernetes/Redis/Python/Docker', 'strengthened single-point
no-numbering rule', 'reinforced informal-tone preservation in
structuring module']

Verification:
- cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture
  → 200 / 200 pass
- cargo test --lib → 227 pass (no regression in unit tests)
- pnpm build → ok

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Final Verification + Doc Map Update

**Files:**
- Modify: `CLAUDE.md` — Documentation Map row + 「最后校验」 date

- [ ] **Step 1: Re-run full unit suite to confirm zero regression**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -10
```

Expected: 227 passed, 0 failed (225 existing + 2 new from Task 1).

- [ ] **Step 2: Re-run frontend build**

```bash
cd /Users/zhenghui/Documents/repos/input0 && pnpm build 2>&1 | tail -5
```

Expected: TypeScript + Vite build succeeds.

- [ ] **Step 3: Re-run eval one more time end-to-end (stability check)**

```bash
cd src-tauri && cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture 2>&1 | tail -30
```

Expected: 200 / 200. If this run fails on any case, the prompt is not robust to temperature=0 variance — return to Task 9 Step 6 flake handling.

- [ ] **Step 4: Update `CLAUDE.md` Documentation Map**

Edit `CLAUDE.md`. Find the Documentation Map table. Add a new row (alphabetically grouped near other prompt features):

```markdown
| [docs/feature-prompt-eval-suite.md](docs/feature-prompt-eval-suite.md) | 200 条提示词评估套件 + 默认提示词重优化（Rust integration test + Codex judge + 100% 通过验收） | 2026-05-10 |
```

Also bump the「最后校验」date for `docs/feature-prompt-optimization.md` to today (2026-05-10) since the prompts described there were materially changed in Task 9.

- [ ] **Step 5: Update spec doc status**

Edit `docs/feature-prompt-eval-suite.md`. Change line 3 from:

```markdown
## 状态：设计中 🛠
```

to:

```markdown
## 状态：已完成 ✅
```

Append a section at the bottom documenting actual outcomes:

```markdown
## 验证结果（2026-05-10）

- `cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture`: **200 / 200 pass** ✅
- `cargo test --lib`: 227 passed / 0 failed
- `pnpm build`: TypeScript strict + Vite build 通过
- 实际迭代轮数：N 轮（替换为真实数字）
- 最终 prompt 调整摘要：
  - [列出 3-5 处实质改动]
- 累计 API 花费：~$X.XX
```

- [ ] **Step 6: Final commit**

```bash
git add CLAUDE.md docs/feature-prompt-eval-suite.md
git commit -m "docs: record prompt eval suite completion in doc map

200/200 verified; doc map updated; spec status flipped to completed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Verification Checklist

- [ ] `cargo test --test prompt_eval -- --include-ignored full_prompt_eval --nocapture` → exits 0 (heuristic 200/200)
- [ ] All rubric cases judge-pass:
  ```bash
  jq '[.cases[] | select(.judge_rubric != null and .judge_result != true) | .id]' tmp/prompt_eval_report.json
  # Must return: []
  ```
- [ ] `cargo test --lib` → 227 passed / 0 failed
- [ ] `pnpm build` → green
- [ ] `tmp/prompt_eval_report.json` exists with 200 cases, all `heuristic.pass=true`, all rubric cases `judge_result=true`
- [ ] `docs/feature-prompt-eval-suite.md` status = 已完成
- [ ] `CLAUDE.md` doc map has new row
- [ ] No changes to `pipeline.rs`, `commands/`, frontend, or any other production-runtime path
