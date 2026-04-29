use crate::llm::client::HistoryEntry;
use std::sync::OnceLock;

pub struct TemplateContext<'a> {
    pub clipboard: Option<&'a str>,
    pub vocabulary: &'a [String],
    pub user_tags: &'a [String],
    pub active_app: Option<&'a str>,
    pub language: &'a str,
    pub history: &'a [HistoryEntry],
}

const CLIPBOARD_LIMIT: usize = 500;

static TEMPLATE_RE: OnceLock<regex::Regex> = OnceLock::new();

fn template_re() -> &'static regex::Regex {
    TEMPLATE_RE.get_or_init(|| regex::Regex::new(r"\{\{(\w+)\}\}").expect("hardcoded regex compiles"))
}

pub fn render_template(template: &str, ctx: &TemplateContext) -> String {
    let re = template_re();
    re.replace_all(template, |caps: &regex::Captures| {
        let name = &caps[1];
        match name {
            "clipboard" => render_clipboard(ctx.clipboard),
            "vocabulary" => render_vocabulary(ctx.vocabulary, ctx.language),
            "user_tags" => ctx.user_tags.join(", "),
            "active_app" => ctx.active_app.unwrap_or("").to_string(),
            "language" => ctx.language.to_string(),
            "history" => render_history(ctx.history),
            _ => caps[0].to_string(), // unknown tag: keep verbatim
        }
    })
    .into_owned()
}

fn render_clipboard(clip: Option<&str>) -> String {
    match clip {
        Some(s) if !s.is_empty() => {
            if s.chars().count() > CLIPBOARD_LIMIT {
                let truncated: String = s.chars().take(CLIPBOARD_LIMIT).collect();
                format!("{}…", truncated)
            } else {
                s.to_string()
            }
        }
        _ => String::new(),
    }
}

fn render_vocabulary(vocab: &[String], language: &str) -> String {
    if vocab.is_empty() {
        return String::new();
    }
    let sep = if language == "zh" { "、" } else { ", " };
    vocab.join(sep)
}

fn render_history(history: &[HistoryEntry]) -> String {
    if history.is_empty() {
        return String::new();
    }
    let skip = history.len().saturating_sub(crate::llm::client::MAX_HISTORY_CONTEXT);
    history
        .iter()
        .skip(skip)
        .enumerate()
        .map(|(i, entry)| format!("{}. STT: {} → Corrected: {}", i + 1, entry.original, entry.corrected))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_ctx<'a>() -> TemplateContext<'a> {
        TemplateContext {
            clipboard: None,
            vocabulary: &[],
            user_tags: &[],
            active_app: None,
            language: "en",
            history: &[],
        }
    }

    #[test]
    fn test_render_no_tags_passthrough() {
        let ctx = empty_ctx();
        assert_eq!(render_template("plain text", &ctx), "plain text");
    }

    #[test]
    fn test_clipboard_tag_some() {
        let mut ctx = empty_ctx();
        ctx.clipboard = Some("hello world");
        assert_eq!(render_template("ctx: {{clipboard}}", &ctx), "ctx: hello world");
    }

    #[test]
    fn test_clipboard_tag_none_renders_empty() {
        let ctx = empty_ctx();
        assert_eq!(render_template("ctx: [{{clipboard}}]", &ctx), "ctx: []");
    }

    #[test]
    fn test_clipboard_tag_truncated() {
        let s: String = "a".repeat(600);
        let mut ctx = empty_ctx();
        ctx.clipboard = Some(&s);
        let out = render_template("{{clipboard}}", &ctx);
        assert_eq!(out.chars().count(), 501, "500 chars + ellipsis");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn test_vocabulary_tag_zh_separator() {
        let vocab = vec!["React".to_string(), "TypeScript".to_string()];
        let mut ctx = empty_ctx();
        ctx.vocabulary = &vocab;
        ctx.language = "zh";
        assert_eq!(render_template("{{vocabulary}}", &ctx), "React、TypeScript");
    }

    #[test]
    fn test_vocabulary_tag_en_separator() {
        let vocab = vec!["React".to_string(), "TypeScript".to_string()];
        let mut ctx = empty_ctx();
        ctx.vocabulary = &vocab;
        ctx.language = "en";
        assert_eq!(render_template("{{vocabulary}}", &ctx), "React, TypeScript");
    }

    #[test]
    fn test_user_tags_tag() {
        let tags = vec!["Developer".to_string(), "Frontend".to_string()];
        let mut ctx = empty_ctx();
        ctx.user_tags = &tags;
        assert_eq!(render_template("{{user_tags}}", &ctx), "Developer, Frontend");
    }

    #[test]
    fn test_active_app_tag() {
        let mut ctx = empty_ctx();
        ctx.active_app = Some("VS Code");
        assert_eq!(render_template("App: {{active_app}}", &ctx), "App: VS Code");
    }

    #[test]
    fn test_language_tag() {
        let mut ctx = empty_ctx();
        ctx.language = "zh";
        assert_eq!(render_template("Lang={{language}}", &ctx), "Lang=zh");
    }

    #[test]
    fn test_history_tag_renders_recent_entries() {
        let history = vec![
            HistoryEntry { original: "raw1".into(), corrected: "fix1".into() },
            HistoryEntry { original: "raw2".into(), corrected: "fix2".into() },
        ];
        let mut ctx = empty_ctx();
        ctx.history = &history;
        let out = render_template("{{history}}", &ctx);
        assert!(out.contains("1. STT: raw1 → Corrected: fix1"));
        assert!(out.contains("2. STT: raw2 → Corrected: fix2"));
    }

    #[test]
    fn test_history_tag_truncates_to_last_n() {
        let history: Vec<HistoryEntry> = (0..15)
            .map(|i| HistoryEntry { original: format!("o{}", i), corrected: format!("c{}", i) })
            .collect();
        let mut ctx = empty_ctx();
        ctx.history = &history;
        let out = render_template("{{history}}", &ctx);
        assert!(out.contains("c14"), "should include last entry");
        assert!(!out.contains("c4"), "should drop entry index 4 (only last 10)");
    }

    #[test]
    fn test_unknown_tag_kept_verbatim() {
        let ctx = empty_ctx();
        assert_eq!(render_template("hello {{unknown}} world", &ctx), "hello {{unknown}} world");
    }

    #[test]
    fn test_multiple_tags_substituted_in_one_pass() {
        let vocab = vec!["A".to_string()];
        let tags = vec!["X".to_string()];
        let mut ctx = empty_ctx();
        ctx.vocabulary = &vocab;
        ctx.user_tags = &tags;
        ctx.active_app = Some("App");
        ctx.language = "zh";
        let out = render_template("[{{vocabulary}}][{{user_tags}}][{{active_app}}][{{language}}]", &ctx);
        assert_eq!(out, "[A][X][App][zh]");
    }

    #[test]
    fn test_adjacent_tags() {
        let mut ctx = empty_ctx();
        ctx.active_app = Some("X");
        ctx.language = "en";
        assert_eq!(render_template("{{active_app}}{{language}}", &ctx), "Xen");
    }
}
