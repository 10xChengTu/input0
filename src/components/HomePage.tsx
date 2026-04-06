import { useState, useCallback } from "react";
import { motion } from "framer-motion";
import { useSettingsStore } from "../stores/settings-store";
import { useRecordingStore } from "../stores/recording-store";
import { useVocabularyStore } from "../stores/vocabulary-store";
import { useLocaleStore } from "../i18n";
import { detectReplacements } from "../utils/word-diff";
import { OnboardingGuide } from "./OnboardingGuide";

interface HomePageProps {
  onNavigateToSettings: (section: string) => void;
  onToast: (message: string, type: "success" | "error") => void;
}

export function HomePage({ onNavigateToSettings, onToast }: HomePageProps) {
  const { isModelLoaded, sttModels, language, hotkey, modelRecommendation, onboardingCompleted } = useSettingsStore();
  const { lastOptimizedText, clearLastResult } = useRecordingStore();
  const { validateAndAdd, loadVocabulary } = useVocabularyStore();
  const { t, locale } = useLocaleStore();
  const activeModel = sttModels.find((m) => m.is_active);

  const [editedText, setEditedText] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [showCorrection, setShowCorrection] = useState(false);

  const handleStartCorrection = useCallback(() => {
    setEditedText(lastOptimizedText);
    setShowCorrection(true);
  }, [lastOptimizedText]);

  const handleDismiss = useCallback(() => {
    setShowCorrection(false);
    clearLastResult();
  }, [clearLastResult]);

  const handleSubmitCorrection = useCallback(async () => {
    const replacements = detectReplacements(lastOptimizedText, editedText);
    if (replacements.length === 0) {
      onToast(t.home.correctionNone, "error");
      return;
    }

    setIsSubmitting(true);
    let learnedCount = 0;
    let failedCount = 0;
    let noApiKey = false;

    for (const r of replacements) {
      try {
        const valid = await validateAndAdd(r.original, r.correct);
        if (valid) learnedCount++;
        else failedCount++;
      } catch (err: unknown) {
        failedCount++;
        if (typeof err === "string" && err.includes("API Key")) {
          noApiKey = true;
        }
      }
    }

    setIsSubmitting(false);

    if (noApiKey && learnedCount === 0) {
      onToast(t.home.correctionNoApiKey, "error");
    } else if (learnedCount > 0 && failedCount > 0) {
      await loadVocabulary();
      onToast(t.home.correctionPartial(learnedCount, replacements.length), "success");
      setShowCorrection(false);
      clearLastResult();
    } else if (learnedCount > 0) {
      await loadVocabulary();
      onToast(t.home.correctionLearned(learnedCount), "success");
      setShowCorrection(false);
      clearLastResult();
    } else {
      onToast(t.home.correctionFailed, "error");
    }
  }, [lastOptimizedText, editedText, validateAndAdd, loadVocabulary, onToast, t, clearLastResult]);

  const languageLabels: Record<string, string> = {
    auto: locale === "zh" ? "自动检测" : "Auto Detect",
    zh: "中文",
    en: "English",
    ja: "日本語",
    ko: "한국어",
    es: "Español",
    fr: "Français",
    de: "Deutsch",
  };

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
      >
        <h2 className="text-xl font-semibold text-[var(--theme-on-surface)] tracking-tight">
          {t.home.welcome}
        </h2>
        <p className="text-sm text-[var(--theme-on-surface-variant)] mt-1">
          {t.home.subtitle}
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.05 }}
        className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-5 cursor-pointer"
        onClick={() => onNavigateToSettings("stt-model")}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="relative flex h-3 w-3">
              {isModelLoaded ? (
                <>
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--theme-status-ping)] opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-3 w-3 bg-[var(--theme-status-dot-loaded)]"></span>
                </>
              ) : (
                <span className="relative inline-flex rounded-full h-3 w-3 bg-[var(--theme-status-dot-unloaded)]"></span>
              )}
            </div>
            <div>
              <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">
                {activeModel?.display_name || t.home.modelLabel}
              </h3>
              <p className="text-xs text-[var(--theme-on-surface-variant)] mt-0.5">
                {isModelLoaded ? t.home.modelLoaded : t.home.modelNotLoaded}
              </p>
            </div>
          </div>
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-outline)]">
            <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          </svg>
        </div>
      </motion.div>

      {modelRecommendation && (
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: 0.08 }}
          className="bg-[var(--theme-reco-bg)] border border-[var(--theme-reco-border)] rounded-xl p-4"
        >
          <div className="flex items-center gap-3">
            <div className="flex-shrink-0">
              <svg className="h-4 w-4 text-[var(--theme-reco-icon)]" viewBox="0 0 20 20" fill="currentColor">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
              </svg>
            </div>
            <p className="flex-1 min-w-0 text-sm text-[var(--theme-on-surface-variant)]">
              {t.home.recommendMessage}
            </p>
            <button
              type="button"
              onClick={() => onNavigateToSettings("stt-model")}
              className="flex-shrink-0 inline-flex items-center gap-1 px-3 py-1.5 rounded-lg text-xs font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] border border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)] transition-colors"
            >
              {t.home.switchModel}
            </button>
          </div>
        </motion.div>
      )}

      {lastOptimizedText && (
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: 0.09 }}
          className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden"
        >
          <div className="px-5 pt-4 pb-2 flex items-center justify-between">
            <div>
              <h3 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider">
                {t.home.lastResult}
              </h3>
              <p className="text-xs text-[var(--theme-on-surface-variant)] mt-0.5">
                {t.home.lastResultHint}
              </p>
            </div>
            <button
              type="button"
              onClick={handleDismiss}
              className="text-[var(--theme-outline)] hover:text-[var(--theme-on-surface)] transition-colors p-1"
            >
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4">
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {showCorrection ? (
            <div className="px-5 pb-4">
              <textarea
                value={editedText}
                onChange={(e) => setEditedText(e.target.value)}
                rows={3}
                disabled={isSubmitting}
                className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-sm text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow resize-y"
              />
              <div className="flex items-center justify-end gap-2 mt-3">
                <button
                  type="button"
                  onClick={handleDismiss}
                  disabled={isSubmitting}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--theme-on-surface-variant)] hover:bg-[var(--theme-surface)] transition-colors"
                >
                  {t.home.dismissResult}
                </button>
                <button
                  type="button"
                  onClick={handleSubmitCorrection}
                  disabled={isSubmitting || editedText === lastOptimizedText}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] border border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isSubmitting ? t.home.submitting : t.home.submitCorrection}
                </button>
              </div>
            </div>
          ) : (
            <div className="px-5 pb-4">
              <p className="text-sm text-[var(--theme-on-surface)] leading-relaxed whitespace-pre-wrap">
                {lastOptimizedText}
              </p>
              <div className="flex justify-end mt-3">
                <button
                  type="button"
                  onClick={handleStartCorrection}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] border border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)] transition-colors"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3.5 h-3.5">
                    <path strokeLinecap="round" strokeLinejoin="round" d="m16.862 4.487 1.687-1.688a1.875 1.875 0 1 1 2.652 2.652L10.582 16.07a4.5 4.5 0 0 1-1.897 1.13L6 18l.8-2.685a4.5 4.5 0 0 1 1.13-1.897l8.932-8.931Zm0 0L19.5 7.125M18 14v4.75A2.25 2.25 0 0 1 15.75 21H5.25A2.25 2.25 0 0 1 3 18.75V8.25A2.25 2.25 0 0 1 5.25 6H10" />
                  </svg>
                  {t.home.editAndSubmit}
                </button>
              </div>
            </div>
          )}
        </motion.div>
      )}

      <div className="grid grid-cols-2 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: 0.1 }}
          className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-4 cursor-pointer relative"
          onClick={() => onNavigateToSettings("hotkey")}
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3.5 h-3.5 text-[var(--theme-outline)] absolute top-3.5 right-3.5">
            <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          </svg>
          <div className="flex items-center gap-2 mb-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-outline)]">
              <path strokeLinecap="round" strokeLinejoin="round" d="M6.75 7.5l3 2.25-3 2.25m4.5 0h3m-9 8.25h13.5A2.25 2.25 0 0021 18V6a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 6v12a2.25 2.25 0 002.25 2.25z" />
            </svg>
            <span className="text-xs font-medium text-[var(--theme-on-surface-variant)]">{t.home.hotkey}</span>
          </div>
          <kbd className="inline-flex items-center px-2.5 py-1 rounded-md border border-[var(--theme-kbd-border)] bg-[var(--theme-kbd-bg)] text-sm font-sans font-medium text-[var(--theme-kbd-text)]">
            {hotkey || "Option+Space"}
          </kbd>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: 0.15 }}
          className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-4 cursor-pointer relative"
          onClick={() => onNavigateToSettings("language")}
        >
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3.5 h-3.5 text-[var(--theme-outline)] absolute top-3.5 right-3.5">
            <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
          </svg>
          <div className="flex items-center gap-2 mb-2">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-outline)]">
              <path strokeLinecap="round" strokeLinejoin="round" d="M10.5 21l5.25-11.25L21 21m-9-3h7.5M3 5.621a48.474 48.474 0 016-.371m0 0c1.12 0 2.233.038 3.334.114M9 5.25V3m3.334 2.364C11.176 10.658 7.69 15.08 3 17.502m9.334-12.138c.896.061 1.785.147 2.666.257m-4.589 8.495a18.023 18.023 0 01-3.827-5.802" />
            </svg>
            <span className="text-xs font-medium text-[var(--theme-on-surface-variant)]">{t.home.language}</span>
          </div>
          <span className="text-sm font-medium text-[var(--theme-on-surface)]">
            {languageLabels[language] || language}
          </span>
        </motion.div>
      </div>

      {onboardingCompleted ? (
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3, delay: 0.2 }}
          className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-4"
        >
          <h3 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-3">{t.home.usageTitle}</h3>
          <div className="space-y-2.5">
            <div className="flex items-start gap-3">
              <span className="flex-shrink-0 w-5 h-5 rounded-full bg-[var(--theme-tag-bg)] border border-[var(--theme-tag-border)] flex items-center justify-center text-[10px] font-semibold text-[var(--theme-on-surface-variant)]">1</span>
              <p className="text-sm text-[var(--theme-on-surface-variant)]">{t.home.usageStep1Prefix}<kbd className="px-1.5 py-0.5 rounded border border-[var(--theme-kbd-border)] bg-[var(--theme-kbd-bg)] text-xs font-medium text-[var(--theme-kbd-text)]">{hotkey || "Option+Space"}</kbd>{t.home.usageStep1Suffix}</p>
            </div>
            <div className="flex items-start gap-3">
              <span className="flex-shrink-0 w-5 h-5 rounded-full bg-[var(--theme-tag-bg)] border border-[var(--theme-tag-border)] flex items-center justify-center text-[10px] font-semibold text-[var(--theme-on-surface-variant)]">2</span>
              <p className="text-sm text-[var(--theme-on-surface-variant)]">{t.home.usageStep2}</p>
            </div>
            <div className="flex items-start gap-3">
              <span className="flex-shrink-0 w-5 h-5 rounded-full bg-[var(--theme-tag-bg)] border border-[var(--theme-tag-border)] flex items-center justify-center text-[10px] font-semibold text-[var(--theme-on-surface-variant)]">3</span>
              <p className="text-sm text-[var(--theme-on-surface-variant)]">{t.home.usageStep3}</p>
            </div>
          </div>
        </motion.div>
      ) : (
        <OnboardingGuide onNavigateToSettings={onNavigateToSettings} />
      )}
    </div>
  );
}
