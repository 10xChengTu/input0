import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../stores/settings-store";
import { useLocaleStore } from "../i18n";

const TAGS = ["clipboard", "vocabulary", "user_tags", "active_app", "language", "history"] as const;
type TagName = (typeof TAGS)[number];

interface Props {
  onToast: (message: string, type: "success" | "error") => void;
}

export function CustomPromptPanel({ onToast }: Props) {
  const { t } = useLocaleStore();
  const {
    customPromptEnabled,
    customPrompt,
    language,
    setCustomPromptEnabled,
    setCustomPrompt,
    saveField,
  } = useSettingsStore();

  const [defaultTemplate, setDefaultTemplate] = useState("");
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewContent, setPreviewContent] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const debounceRef = useRef<number | null>(null);

  // Fetch language-aware default once when language changes (and on mount).
  useEffect(() => {
    invoke<string>("get_default_prompt_template", { language })
      .then(setDefaultTemplate)
      .catch((err) => console.error("Failed to load default prompt template:", err));
  }, [language]);

  // Cancel any pending debounced save when the panel unmounts.
  useEffect(() => {
    return () => {
      if (debounceRef.current !== null) {
        window.clearTimeout(debounceRef.current);
      }
    };
  }, []);

  const persistEnabled = async (enabled: boolean) => {
    setCustomPromptEnabled(enabled);
    try {
      await saveField("custom_prompt_enabled", enabled ? "true" : "false");
    } catch {
      onToast(t.settings.settingsSaveFailed, "error");
    }
  };

  const persistPrompt = (next: string) => {
    setCustomPrompt(next);
    if (debounceRef.current !== null) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      saveField("custom_prompt", next).catch(() => {
        onToast(t.settings.settingsSaveFailed, "error");
      });
    }, 500);
  };

  const insertTag = (tag: TagName) => {
    const ta = textareaRef.current;
    if (!ta) return;
    const value = ta.value;
    const start = ta.selectionStart ?? value.length;
    const end = ta.selectionEnd ?? value.length;
    const insert = `{{${tag}}}`;
    const next = value.slice(0, start) + insert + value.slice(end);
    persistPrompt(next);
    requestAnimationFrame(() => {
      ta.focus();
      const caret = start + insert.length;
      ta.setSelectionRange(caret, caret);
    });
  };

  const handleReset = async () => {
    const ok = window.confirm(t.settings.customPromptResetConfirm);
    if (!ok) return;
    persistPrompt("");
    onToast(t.settings.settingsSaved, "success");
  };

  const handlePreview = async () => {
    try {
      const template = customPrompt.trim().length > 0 ? customPrompt : defaultTemplate;
      const rendered = await invoke<string>("preview_custom_prompt", { template });
      setPreviewContent(rendered);
      setPreviewOpen(true);
    } catch (err) {
      onToast(String(err), "error");
    }
  };

  const displayValue = customPrompt.length > 0 ? customPrompt : defaultTemplate;
  const tagDescriptions = t.settings.customPromptTagDescriptions;

  return (
    <section className="space-y-6">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.customPromptTitle}</h2>
          <p className="text-xs text-[var(--theme-on-surface-variant)] mt-1">{t.settings.customPromptDescription}</p>
        </div>
        <label className="inline-flex items-center cursor-pointer flex-shrink-0">
          <input
            type="checkbox"
            className="sr-only peer"
            checked={customPromptEnabled}
            onChange={(e) => persistEnabled(e.target.checked)}
          />
          <span className="relative w-10 h-6 bg-[var(--theme-surface-container)] rounded-full peer-checked:bg-[var(--theme-primary)] transition-colors">
            <span className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${customPromptEnabled ? "translate-x-4" : ""}`} />
          </span>
          <span className="ml-2 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.customPromptEnableLabel}</span>
        </label>
      </header>

      <div>
        <p className="text-xs text-[var(--theme-on-surface-variant)] mb-2">{t.settings.customPromptInsertTagLabel}</p>
        <div className="flex flex-wrap gap-2">
          {TAGS.map((tag) => (
            <button
              key={tag}
              type="button"
              onClick={() => insertTag(tag)}
              title={tagDescriptions[tag]}
              className="px-2.5 py-1 text-xs font-mono rounded-md bg-[var(--theme-surface-container)] hover:bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)] transition-colors"
            >{`{{${tag}}}`}</button>
          ))}
        </div>
      </div>

      <textarea
        ref={textareaRef}
        value={displayValue}
        onChange={(e) => persistPrompt(e.target.value)}
        rows={20}
        spellCheck={false}
        className="w-full p-3 rounded-md bg-[var(--theme-surface-container)] text-[var(--theme-on-surface)] text-[13px] font-mono leading-relaxed outline-none focus:ring-2 focus:ring-[var(--theme-primary)]"
      />
      <p className="text-[11px] text-[var(--theme-on-surface-variant)]">{displayValue.length} {t.settings.customPromptCharsLabel}</p>

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={handleReset}
          className="px-3 py-1.5 text-xs rounded-md bg-[var(--theme-surface-container)] hover:bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)]"
        >
          {t.settings.customPromptResetToDefault}
        </button>
        <button
          type="button"
          onClick={handlePreview}
          className="px-3 py-1.5 text-xs rounded-md bg-[var(--theme-primary)] hover:opacity-90 text-white"
        >
          {t.settings.customPromptPreview}
        </button>
      </div>

      {previewOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
          onClick={() => setPreviewOpen(false)}
        >
          <div
            className="bg-[var(--theme-surface)] rounded-xl p-5 max-w-[720px] w-[90vw] max-h-[80vh] overflow-auto"
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className="text-sm font-medium text-[var(--theme-on-surface)] mb-3">{t.settings.customPromptPreviewModalTitle}</h3>
            <pre className="whitespace-pre-wrap text-[12px] font-mono text-[var(--theme-on-surface-variant)]">{previewContent}</pre>
            <button
              type="button"
              onClick={() => setPreviewOpen(false)}
              className="mt-4 px-3 py-1.5 text-xs rounded-md bg-[var(--theme-surface-container)] hover:bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)]"
            >{t.settings.customPromptPreviewClose}</button>
          </div>
        </div>
      )}
    </section>
  );
}
