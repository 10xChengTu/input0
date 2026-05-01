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
    textStructuring,
    structuringPrompt,
    setCustomPromptEnabled,
    setCustomPrompt,
    setTextStructuring,
    setStructuringPrompt,
    saveField,
  } = useSettingsStore();

  const [defaultTemplate, setDefaultTemplate] = useState("");
  const [defaultStructuringModule, setDefaultStructuringModule] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const customDebounceRef = useRef<number | null>(null);
  const structuringDebounceRef = useRef<number | null>(null);

  useEffect(() => {
    invoke<string>("get_default_prompt_template", { language })
      .then(setDefaultTemplate)
      .catch((err) => console.error("Failed to load default prompt template:", err));
    invoke<string>("get_default_structuring_module", { language })
      .then(setDefaultStructuringModule)
      .catch((err) => console.error("Failed to load default structuring module:", err));
  }, [language]);

  useEffect(() => {
    return () => {
      if (customDebounceRef.current !== null) window.clearTimeout(customDebounceRef.current);
      if (structuringDebounceRef.current !== null) window.clearTimeout(structuringDebounceRef.current);
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

  const persistStructuring = async (enabled: boolean) => {
    setTextStructuring(enabled);
    try {
      await saveField("text_structuring", String(enabled));
    } catch {
      setTextStructuring(!enabled);
      onToast(t.settings.settingsSaveFailed, "error");
    }
  };

  const persistPrompt = (next: string) => {
    setCustomPrompt(next);
    if (customDebounceRef.current !== null) window.clearTimeout(customDebounceRef.current);
    customDebounceRef.current = window.setTimeout(() => {
      saveField("custom_prompt", next).catch(() => onToast(t.settings.settingsSaveFailed, "error"));
    }, 500);
  };

  const persistStructuringPrompt = (next: string) => {
    setStructuringPrompt(next);
    if (structuringDebounceRef.current !== null) window.clearTimeout(structuringDebounceRef.current);
    structuringDebounceRef.current = window.setTimeout(() => {
      saveField("structuring_prompt", next).catch(() => onToast(t.settings.settingsSaveFailed, "error"));
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

  const handleStructuringReset = async () => {
    const ok = window.confirm(t.settings.customPromptResetConfirm);
    if (!ok) return;
    persistStructuringPrompt("");
    onToast(t.settings.settingsSaved, "success");
  };

  const displayValue = customPrompt.length > 0 ? customPrompt : defaultTemplate;
  const structuringDisplayValue = structuringPrompt.length > 0 ? structuringPrompt : defaultStructuringModule;
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
      <div className="flex items-center justify-between">
        <p className="text-[11px] text-[var(--theme-on-surface-variant)]">{displayValue.length} {t.settings.customPromptCharsLabel}</p>
        <button
          type="button"
          onClick={handleReset}
          className="px-3 py-1.5 text-xs rounded-md bg-[var(--theme-surface-container)] hover:bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)]"
        >
          {t.settings.customPromptResetToDefault}
        </button>
      </div>

      <hr className="border-[var(--theme-divider)]" />

      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.textStructuringLabel}</h3>
          <p className="text-xs text-[var(--theme-on-surface-variant)] mt-1">{t.settings.textStructuringHint}</p>
        </div>
        <label className="inline-flex items-center cursor-pointer flex-shrink-0">
          <input
            type="checkbox"
            className="sr-only peer"
            checked={textStructuring}
            onChange={(e) => persistStructuring(e.target.checked)}
          />
          <span className="relative w-10 h-6 bg-[var(--theme-surface-container)] rounded-full peer-checked:bg-[var(--theme-primary)] transition-colors">
            <span className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${textStructuring ? "translate-x-4" : ""}`} />
          </span>
        </label>
      </div>

      <textarea
        value={structuringDisplayValue}
        onChange={(e) => persistStructuringPrompt(e.target.value)}
        rows={12}
        spellCheck={false}
        disabled={!textStructuring}
        className={`w-full p-3 rounded-md bg-[var(--theme-surface-container)] text-[var(--theme-on-surface)] text-[13px] font-mono leading-relaxed outline-none focus:ring-2 focus:ring-[var(--theme-primary)] ${textStructuring ? "" : "opacity-50"}`}
      />
      <div className="flex items-center justify-between">
        <p className="text-[11px] text-[var(--theme-on-surface-variant)]">{structuringDisplayValue.length} {t.settings.customPromptCharsLabel}</p>
        <button
          type="button"
          onClick={handleStructuringReset}
          className="px-3 py-1.5 text-xs rounded-md bg-[var(--theme-surface-container)] hover:bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)]"
        >
          {t.settings.customPromptResetToDefault}
        </button>
      </div>
    </section>
  );
}
