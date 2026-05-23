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
