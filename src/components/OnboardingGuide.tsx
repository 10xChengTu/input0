import { useCallback } from "react";
import { motion } from "framer-motion";
import { useSettingsStore } from "../stores/settings-store";
import { useLocaleStore } from "../i18n";

interface OnboardingGuideProps {
  onNavigateToSettings: (section: string) => void;
}

export function OnboardingGuide({ onNavigateToSettings }: OnboardingGuideProps) {
  const { apiKey, sttModels, userTags, completeOnboarding } = useSettingsStore();
  const { t } = useLocaleStore();

  const hasModel = sttModels.some((m) => m.is_downloaded);
  const hasApiKey = apiKey.trim().length > 0;
  const hasTags = userTags.length > 0;
  const allDone = hasModel && hasApiKey && hasTags;

  const handleComplete = useCallback(async () => {
    await completeOnboarding();
  }, [completeOnboarding]);

  const steps = [
    {
      done: hasModel,
      label: hasModel ? t.onboarding.stepModelDone : t.onboarding.stepModel,
      hint: t.onboarding.stepModelHint,
      section: "stt-model",
    },
    {
      done: hasApiKey,
      label: hasApiKey ? t.onboarding.stepApiKeyDone : t.onboarding.stepApiKey,
      hint: t.onboarding.stepApiKeyHint,
      section: "api-key",
    },
    {
      done: hasTags,
      label: hasTags ? t.onboarding.stepUserTagsDone : t.onboarding.stepUserTags,
      hint: t.onboarding.stepUserTagsHint,
      section: "user-tags",
    },
  ];

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, delay: 0.1 }}
      className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-5"
    >
      <h3 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-1">
        {t.onboarding.title}
      </h3>
      <p className="text-xs text-[var(--theme-on-surface-variant)] mb-4">
        {allDone ? t.onboarding.allDone : t.onboarding.subtitle}
      </p>

      <div className="space-y-2">
        {steps.map((step) => (
          <div
            key={step.section}
            onClick={() => {
              if (!step.done) onNavigateToSettings(step.section);
            }}
            className={`flex items-center gap-3 rounded-lg px-4 py-3 border transition-colors ${
              step.done
                ? "border-[var(--theme-outline-variant)] bg-[var(--theme-surface-container-lowest)]"
                : "border-[var(--theme-outline-variant)] bg-[var(--theme-surface-container-lowest)] cursor-pointer hover:bg-[var(--theme-surface)]"
            }`}
          >
            <div className="flex-shrink-0">
              {step.done ? (
                <div className="w-5 h-5 rounded-full bg-[var(--theme-status-dot-loaded)] flex items-center justify-center">
                  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2.5} stroke="white" className="w-3 h-3">
                    <path strokeLinecap="round" strokeLinejoin="round" d="m4.5 12.75 6 6 9-13.5" />
                  </svg>
                </div>
              ) : (
                <div className="w-5 h-5 rounded-full border-2 border-[var(--theme-outline-variant)]" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <span className={`text-sm font-medium ${
                step.done
                  ? "text-[var(--theme-on-surface-variant)] line-through"
                  : "text-[var(--theme-on-surface)]"
              }`}>
                {step.label}
              </span>
              {!step.done && (
                <p className="text-xs text-[var(--theme-on-surface-variant)] mt-0.5">
                  {step.hint}
                </p>
              )}
            </div>
            {!step.done && (
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-outline)] flex-shrink-0">
                <path strokeLinecap="round" strokeLinejoin="round" d="m8.25 4.5 7.5 7.5-7.5 7.5" />
              </svg>
            )}
          </div>
        ))}
      </div>

      {allDone && (
        <motion.div
          initial={{ opacity: 0, y: 4 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.2 }}
          className="mt-4 flex justify-end"
        >
          <button
            type="button"
            onClick={handleComplete}
            className="milled-button inline-flex items-center px-4 py-2 rounded-lg text-sm font-medium"
          >
            {t.onboarding.complete}
          </button>
        </motion.div>
      )}
    </motion.div>
  );
}
