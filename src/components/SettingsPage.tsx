import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useSettingsStore } from "../stores/settings-store";
import { useUpdateStore } from "../stores/update-store";
import { motion, AnimatePresence } from "framer-motion";
import { useLocaleStore } from "../i18n";

/** Map KeyboardEvent.code to our app hotkey token (code is layout-independent) */
function codeToHotkeyPart(code: string): string | null {
  if (["MetaLeft", "MetaRight", "AltLeft", "AltRight", "ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight"].includes(code)) return null;
  if (code === "Space") return "Space";
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (code.startsWith("Arrow")) return code.replace("Arrow", "");
  if (code.startsWith("Numpad")) return code;
  const knownKeys: Record<string, string> = {
    Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]",
    Backslash: "\\", Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
    Enter: "Enter", Backspace: "Backspace", Tab: "Tab", Delete: "Delete",
    Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
    F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
    F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
  };
  return knownKeys[code] ?? code;
}

/** Build hotkey string from a KeyboardEvent, e.g. "Option+Space" */
function buildHotkeyString(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Control");
  if (e.altKey) parts.push("Option");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Command");

  const mainKey = codeToHotkeyPart(e.code);
  if (!mainKey) return null;
  parts.push(mainKey);
  return parts.join("+");
}

const EyeIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
    <path strokeLinecap="round" strokeLinejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
  </svg>
);

const EyeSlashIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5">
    <path strokeLinecap="round" strokeLinejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
  </svg>
);

interface HotkeyRecorderProps {
  onCapture: (hotkey: string) => void;
  onCancel: () => void;
  recordingText: string;
}

function HotkeyRecorder({ onCapture, onCancel, recordingText }: HotkeyRecorderProps) {
  const recorderRef = useRef<HTMLDivElement>(null);
  const unregisteredRef = useRef(false);

  useEffect(() => {
    invoke("unregister_hotkey").then(() => {
      unregisteredRef.current = true;
    }).catch(() => {});

    return () => {
      if (unregisteredRef.current) {
        invoke("reregister_hotkey").catch(() => {});
      }
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        onCancel();
        return;
      }

      const combo = buildHotkeyString(e);
      if (combo) {
        unregisteredRef.current = false;
        onCapture(combo);
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [onCapture, onCancel]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (recorderRef.current && !recorderRef.current.contains(e.target as Node)) {
        onCancel();
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [onCancel]);

  return (
    <div
      ref={recorderRef}
      className="inline-flex items-center px-3 py-1.5 rounded-md border-2 border-[var(--theme-primary)] bg-[var(--theme-kbd-bg)] text-sm font-medium text-[var(--theme-primary)] animate-pulse"
    >
      {recordingText}
    </div>
  );
}

type SettingsTab = "general" | "api" | "models";

interface SettingsPageProps {
  onToast: (message: string, type: "success" | "error") => void;
  scrollToSection?: string | null;
  onScrollComplete?: () => void;
}

export function SettingsPage({ onToast, scrollToSection, onScrollComplete }: SettingsPageProps) {
  const {
    apiKey,
    apiBaseUrl,
    model,
    language,
    hotkey,
    modelPath,
    isModelLoaded,
    isTesting,
    testResult,
    setApiKey,
    setApiBaseUrl,
    setModel,
    setLanguage,
    saveField,
    testApiConnection,
    sttModels,
    isLoadingModels,
    downloadingModelId,
    downloadProgress,
    modelRecommendation,
    downloadModel,
    switchModel,
    deleteModel,
    checkModelRecommendation,
    textStructuring,
    setTextStructuring,
    updateHotkey,
    userTags,
    setUserTags,
  } = useSettingsStore();

  const { t } = useLocaleStore();
  const activeModel = sttModels.find((m) => m.is_active);
  const [showApiKey, setShowApiKey] = useState(false);
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const focusValueRef = useRef<string>("");

  const {
    updateAvailable,
    updateVersion,
    updateBody,
    isChecking,
    isDownloading,
    downloadProgress: updateDownloadProgress,
    error: updateError,
    checkForUpdates,
    downloadAndInstall,
    dismissUpdate,
  } = useUpdateStore();

  const [appVersion, setAppVersion] = useState("0.1.0");

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  const sectionToTab = (section: string | null | undefined): SettingsTab => {
    if (section === "stt-model") return "models";
    if (section === "hotkey" || section === "language") return "general";
    if (section === "api-key") return "api";
    if (section === "user-tags") return "general";
    return "general";
  };

  const [activeTab, setActiveTab] = useState<SettingsTab>(
    sectionToTab(scrollToSection)
  );

  useEffect(() => {
    if (scrollToSection) {
      setActiveTab(sectionToTab(scrollToSection));
      onScrollComplete?.();
    }
  }, [scrollToSection, onScrollComplete]);

  const handleFocusCapture = (value: string) => {
    focusValueRef.current = value;
  };

  const handleBlurSave = async (field: string, value: string) => {
    if (value === focusValueRef.current) return;
    try {
      await saveField(field, value);
      onToast(t.settings.settingsSaved, "success");
    } catch {
      onToast(t.settings.settingsSaveFailed, "error");
    }
  };

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: "general", label: t.settings.tabGeneral },
    { id: "api", label: t.settings.tabApi },
    { id: "models", label: t.settings.tabModels },
  ];

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="inline-flex self-start gap-1 p-1 mb-6 bg-[var(--theme-surface-container)] rounded-xl">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-1.5 rounded-[10px] text-sm font-medium transition-all ${
              activeTab === tab.id
                ? "bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)] shadow-sm"
                : "text-[var(--theme-on-surface-variant)] hover:text-[var(--theme-on-surface)]"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div>
        <AnimatePresence mode="popLayout">
          {activeTab === "general" && (
            <motion.div
              key="general"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="space-y-8"
            >
              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.voiceSettingsTitle}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
                  <div className="p-4 sm:p-5">
                    <label htmlFor="language" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.languageLabel}
                    </label>
                    <select
                      id="language"
                      value={language}
                      onChange={(e) => {
                        const newLang = e.target.value;
                        setLanguage(newLang);
                        checkModelRecommendation(newLang);
                        handleBlurSave("language", newLang);
                      }}
                      className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 pl-3 pr-10 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                    >
                      <option value="auto" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.autoDetect}</option>
                      <option value="en" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">English</option>
                      <option value="zh" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">中文 (Chinese)</option>
                      <option value="ja" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">日本語 (Japanese)</option>
                      <option value="ko" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">한국어 (Korean)</option>
                      <option value="es" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">Español (Spanish)</option>
                      <option value="fr" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">Français (French)</option>
                      <option value="de" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">Deutsch (German)</option>
                    </select>
                  </div>

                  <div className="p-4 sm:p-5 flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.hotkeyLabel}</h3>
                      <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.hotkeyHint}</p>
                    </div>
                    <div className="ml-4 flex-shrink-0 flex items-center gap-2">
                      {isRecordingHotkey ? (
                        <HotkeyRecorder
                          onCapture={async (newHotkey) => {
                            setIsRecordingHotkey(false);
                            try {
                              await updateHotkey(newHotkey);
                              onToast(t.settings.hotkeyChanged, "success");
                            } catch {
                              onToast(t.settings.hotkeyChangeFailed, "error");
                            }
                          }}
                          onCancel={() => setIsRecordingHotkey(false)}
                          recordingText={t.settings.hotkeyRecording}
                        />
                      ) : (
                        <>
                          <kbd className="inline-flex items-center px-3 py-1.5 rounded-md border border-[var(--theme-kbd-border)] bg-[var(--theme-kbd-bg)] text-sm font-sans font-medium text-[var(--theme-kbd-text)]">
                            {hotkey || "Option+Space"}
                          </kbd>
                          <button
                            type="button"
                            onClick={() => setIsRecordingHotkey(true)}
                            className="text-xs font-medium text-[var(--theme-primary)] hover:text-[var(--theme-primary-hover)] transition-colors"
                          >
                            {t.settings.hotkeyChange}
                          </button>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="p-4 sm:p-5 flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.textStructuringLabel}</h3>
                      <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.textStructuringHint}</p>
                    </div>
                    <div className="ml-4 flex-shrink-0">
                      <button
                        type="button"
                        role="switch"
                        aria-checked={textStructuring}
                        onClick={async () => {
                          const newValue = !textStructuring;
                          setTextStructuring(newValue);
                          try {
                            await saveField("text_structuring", String(newValue));
                          } catch {
                            setTextStructuring(!newValue);
                            onToast(t.settings.settingsSaveFailed, "error");
                          }
                        }}
                        className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] ${textStructuring ? "bg-[var(--theme-primary)]" : "bg-[var(--theme-outline-variant)]"}`}
                      >
                        <span
                           className={`inline-block h-4 w-4 transform rounded-full shadow transition-all ${textStructuring ? "translate-x-6 bg-[var(--theme-on-primary)]" : "translate-x-1 bg-white"}`}
                        />
                      </button>
                    </div>
                  </div>
                </div>
              </section>

              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.userTagsTitle}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden">
                  <div className="p-4 sm:p-5">
                    <p className="text-xs text-[var(--theme-on-surface-variant)] mb-4">{t.settings.userTagsHint}</p>
                    {[
                      { label: t.settings.userTagsGroupIdentity, tags: ["Developer", "Designer", "Product Manager", "Data Scientist", "DevOps", "Researcher", "Marketer", "Founder", "Student", "Writer", "AI / ML", "Frontend", "Backend", "Mobile", "Cloud", "Security", "Data", "Blockchain", "Game Dev"] },
                      { label: t.settings.userTagsGroupTechStack, tags: ["React", "Vue", "Angular", "Next.js", "TypeScript", "Python", "Rust", "Go", "Java", "Swift", "Kotlin", "C++", "Node.js", "Django", "Spring", "Kubernetes", "Docker", "AWS", "PostgreSQL", "MongoDB", "Redis", "TensorFlow", "PyTorch", "LangChain"] },
                      { label: t.settings.userTagsGroupIndustry, tags: ["Tech", "Finance", "Healthcare", "Education", "Legal", "E-commerce", "Gaming", "Manufacturing", "Media", "Consulting", "Real Estate", "Automotive", "Crypto & Web3", "AI & ML", "Biotech", "Energy", "Aerospace", "Logistics", "Food & Beverage", "Travel & Hospitality", "Fashion", "Sports", "Music & Audio", "Film & Video", "Architecture", "Agriculture"] },
                      { label: t.settings.userTagsGroupUseCase, tags: ["Code Comments", "Emails", "Meeting Notes", "Technical Docs", "Chat Messages", "Blog Posts", "Academic Writing", "Creative Writing", "Social Media Posts", "Product Reviews", "Customer Support", "Project Planning", "Research Notes", "Journaling", "Presentations", "Legal Drafts", "Marketing Copy", "Translations"] },
                    ].map((group) => (
                      <div key={group.label} className="mb-4 last:mb-0">
                        <h3 className="text-xs font-medium text-[var(--theme-on-surface-variant)] mb-2">{group.label}</h3>
                        <div className="flex flex-wrap gap-1.5">
                          {group.tags.map((tag) => {
                            const isSelected = userTags.includes(tag);
                            return (
                              <button
                                key={tag}
                                type="button"
                                onClick={async () => {
                                  const newTags = isSelected
                                    ? userTags.filter((t) => t !== tag)
                                    : [...userTags, tag];
                                  setUserTags(newTags);
                                  try {
                                    await saveField("user_tags", JSON.stringify(newTags));
                                  } catch {
                                    setUserTags(userTags);
                                    onToast(t.settings.settingsSaveFailed, "error");
                                  }
                                }}
                                className={`px-2 py-0.5 rounded text-[10px] font-medium transition-colors ${
                                  isSelected
                                    ? "bg-[var(--theme-primary)] text-[var(--theme-on-primary)] border border-[var(--theme-primary)]"
                                    : "bg-[var(--theme-tag-bg)] text-[var(--theme-tag-text)] border border-[var(--theme-tag-border)] hover:bg-[var(--theme-tag-hover-bg)]"
                                }`}
                              >
                                {t.settings.tagLabels[tag] || tag}
                              </button>
                            );
                          })}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </section>

              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.update.title}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
                  <div className="p-4 sm:p-5 flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.update.currentVersion}</h3>
                      <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">v{appVersion}</p>
                    </div>
                    <button
                      type="button"
                      onClick={checkForUpdates}
                      disabled={isChecking || isDownloading}
                      className="ml-4 inline-flex items-center px-4 py-2 border border-[var(--theme-btn-secondary-border)] rounded-lg text-sm font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {isChecking ? (
                        <>
                          <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-[var(--theme-spinner)]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          {t.update.checking}
                        </>
                      ) : t.update.checkForUpdates}
                    </button>
                  </div>

                  {updateAvailable && updateVersion && (
                    <div className="p-4 sm:p-5">
                      <div className="flex items-start gap-3 mb-3">
                        <div className="flex-shrink-0 mt-0.5">
                          <svg className="h-5 w-5 text-[var(--theme-primary)]" viewBox="0 0 20 20" fill="currentColor">
                            <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm.75-11.25a.75.75 0 00-1.5 0v4.59l-1.95-2.1a.75.75 0 10-1.1 1.02l3.25 3.5a.75.75 0 001.1 0l3.25-3.5a.75.75 0 10-1.1-1.02l-1.95 2.1V6.75z" clipRule="evenodd" />
                          </svg>
                        </div>
                        <div className="flex-1">
                          <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.update.availableMessage(updateVersion)}</h3>
                          {updateBody && (
                            <div className="mt-2 text-xs text-[var(--theme-on-surface-variant)] whitespace-pre-wrap max-h-32 overflow-y-auto">
                              {updateBody}
                            </div>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        {isDownloading ? (
                          <div className="flex-1 flex items-center gap-3">
                            <div className="flex-1 bg-[var(--theme-progress-track)] rounded-full h-1.5 overflow-hidden">
                              <div
                                className="bg-[var(--theme-progress-fill)] h-full rounded-full transition-all duration-300"
                                style={{ width: `${updateDownloadProgress}%` }}
                              />
                            </div>
                            <span className="text-xs text-[var(--theme-on-surface-variant)] font-medium w-10 text-right">{updateDownloadProgress}%</span>
                          </div>
                        ) : (
                          <>
                            <button
                              type="button"
                              onClick={downloadAndInstall}
                              className="milled-button text-sm font-medium px-4 py-2 rounded-lg whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)]"
                            >
                              {t.update.downloadAndInstall}
                            </button>
                            <button
                              type="button"
                              onClick={dismissUpdate}
                              className="text-xs text-[var(--theme-on-surface-variant)] hover:text-[var(--theme-on-surface)] font-medium transition-colors"
                            >
                              {t.update.dismiss}
                            </button>
                          </>
                        )}
                      </div>
                    </div>
                  )}

                  {updateError && (
                    <div className="p-4 sm:p-5">
                      <div className="p-3 rounded-md text-sm bg-[var(--theme-result-error-bg)] text-[var(--theme-result-error-text)] border border-[var(--theme-result-error-border)]">
                        {updateError}
                      </div>
                    </div>
                  )}
                </div>
              </section>
            </motion.div>
          )}

          {activeTab === "api" && (
            <motion.div
              key="api"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="space-y-8"
            >
              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.apiConfigTitle}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
                  <div className="p-4 sm:p-5 relative">
                    <label htmlFor="apiKey" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.apiKeyLabel}
                    </label>
                    <div className="relative rounded-md">
                      <input
                        type={showApiKey ? "text" : "password"}
                        id="apiKey"
                        value={apiKey}
                        onChange={(e) => setApiKey(e.target.value)}
                        onFocus={() => handleFocusCapture(apiKey)}
                        onBlur={() => handleBlurSave("api_key", apiKey)}
                        className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 pl-3 pr-10 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                        placeholder={t.settings.apiKeyPlaceholder}
                      />
                      <button
                        type="button"
                        onClick={() => setShowApiKey(!showApiKey)}
                        className="absolute inset-y-0 right-0 flex items-center pr-3 text-[var(--theme-outline)] hover:text-[var(--theme-on-surface-variant)]"
                      >
                        {showApiKey ? <EyeSlashIcon /> : <EyeIcon />}
                      </button>
                    </div>
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {t.settings.apiKeyHint}
                    </p>
                  </div>

                  <div className="p-4 sm:p-5">
                    <label htmlFor="apiBaseUrl" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.apiBaseUrlLabel}
                    </label>
                    <input
                      type="text"
                      id="apiBaseUrl"
                      value={apiBaseUrl}
                      onChange={(e) => setApiBaseUrl(e.target.value)}
                      onFocus={() => handleFocusCapture(apiBaseUrl)}
                      onBlur={() => handleBlurSave("api_base_url", apiBaseUrl)}
                      className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                      placeholder={t.settings.apiBaseUrlPlaceholder}
                    />
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {t.settings.apiBaseUrlHint}
                    </p>
                  </div>

                  <div className="p-4 sm:p-5">
                    <label htmlFor="model" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.modelLabel}
                    </label>
                    <input
                      type="text"
                      id="model"
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                      onFocus={() => handleFocusCapture(model)}
                      onBlur={() => handleBlurSave("model", model)}
                      className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                      placeholder={t.settings.modelPlaceholder}
                    />
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {t.settings.modelHint}
                    </p>
                  </div>

                  <div className="p-4 sm:p-5">
                    <div className="flex items-center justify-between">
                      <div>
                        <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.testConnection}</h3>
                        <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.testConnectionHint}</p>
                      </div>
                      <button
                        type="button"
                        onClick={testApiConnection}
                        disabled={isTesting || !apiKey}
                        className="ml-4 inline-flex items-center px-4 py-2 border border-[var(--theme-btn-secondary-border)] rounded-lg text-sm font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isTesting ? (
                          <>
                            <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-[var(--theme-spinner)]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                            </svg>
                            {t.settings.testing}
                          </>
                        ) : t.settings.test}
                      </button>
                    </div>
                    {(isTesting || testResult) && (
                      <div className="min-h-[44px] mt-3">
                        <AnimatePresence>
                          {testResult && (
                            <motion.div
                              initial={{ opacity: 0 }}
                              animate={{ opacity: 1 }}
                              exit={{ opacity: 0 }}
                              className={`p-3 rounded-md text-sm ${testResult.success ? "bg-[var(--theme-result-success-bg)] text-[var(--theme-result-success-text)] border border-[var(--theme-result-success-border)]" : "bg-[var(--theme-result-error-bg)] text-[var(--theme-result-error-text)] border border-[var(--theme-result-error-border)]"}`}
                            >
                              {testResult.success ? "✓ " : "✗ "}{testResult.message}
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    )}
                  </div>
                </div>
              </section>
            </motion.div>
          )}

          {activeTab === "models" && (
            <motion.div
              key="models"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="space-y-8"
            >
              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.statusTitle}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-4 sm:p-5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-3">
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
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">
                        {activeModel?.display_name || t.home.modelLabel}
                      </h3>
                    </div>
                    <span className="text-sm text-[var(--theme-on-surface-variant)]">
                      {isModelLoaded ? t.settings.loaded : t.settings.notLoaded}
                    </span>
                  </div>
                  {modelPath && (
                    <div className="mt-4 bg-[var(--theme-code-bg)] rounded-md p-3 border border-[var(--theme-code-border)]">
                      <p className="text-xs text-[var(--theme-on-surface-variant)] font-mono break-all">
                        {modelPath}
                      </p>
                    </div>
                  )}
                </div>
              </section>

              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.sttModelTitle}</h2>
                
                <AnimatePresence>
                  {modelRecommendation && (
                    <motion.div
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: "auto", opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      transition={{ duration: 0.3, ease: "easeInOut" }}
                      className="overflow-hidden"
                    >
                      <div className="mb-4 bg-[var(--theme-reco-bg)] border border-[var(--theme-reco-border)] rounded-xl p-4">
                        <div className="flex items-start gap-3 mb-3">
                          <div className="flex-shrink-0 mt-0.5">
                            <svg className="h-5 w-5 text-[var(--theme-reco-icon)]" viewBox="0 0 20 20" fill="currentColor">
                              <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
                            </svg>
                          </div>
                          <div>
                            <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.recommendTitle}</h3>
                            <p className="text-sm text-[var(--theme-on-surface-variant)] mt-1">
                              {t.settings.recommendMessage}
                            </p>
                          </div>
                        </div>
                        <div className="space-y-2 ml-8">
                          {modelRecommendation.recommended_models.map((rec) => (
                            <div key={rec.id} className="flex items-center justify-between gap-3 bg-[var(--theme-surface-container-lowest)] rounded-lg px-3 py-2 border border-[var(--theme-outline-variant)]">
                              <div className="min-w-0">
                                <span className="text-sm font-semibold text-[var(--theme-on-surface)]">{rec.name}</span>
                                {rec.reason && (
                                  <p className="text-xs text-[var(--theme-on-surface-variant)] mt-0.5 opacity-80">{rec.reason}</p>
                                )}
                              </div>
                              <button
                                type="button"
                                onClick={async () => {
                                  const modelInfo = sttModels.find((m) => m.id === rec.id);
                                  try {
                                    if (!modelInfo?.is_downloaded) {
                                      await downloadModel(rec.id);
                                    }
                                    await switchModel(rec.id);
                                    onToast(t.settings.recommendSwitched, "success");
                                  } catch {
                                    onToast(t.settings.recommendSwitchFailed, "error");
                                  }
                                }}
                                disabled={downloadingModelId !== null}
                                className="flex-shrink-0 px-3 py-1.5 bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] text-[var(--theme-on-surface)] text-xs font-medium rounded-lg border border-[var(--theme-btn-secondary-border)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50"
                              >
                                {downloadingModelId === rec.id ? t.settings.downloading : t.settings.switchLabel}
                              </button>
                            </div>
                          ))}
                        </div>
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>

                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
                  {isLoadingModels ? (
                    <div className="p-4 sm:p-5 flex justify-center">
                      <p className="text-sm text-[var(--theme-on-surface-variant)] flex items-center gap-2">
                        <svg className="animate-spin h-4 w-4 text-[var(--theme-spinner)]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        {t.settings.loadingModels}
                      </p>
                    </div>
                  ) : sttModels.length === 0 ? (
                    <div className="p-4 sm:p-5 flex justify-center">
                      <p className="text-sm text-[var(--theme-on-surface-variant)]">{t.settings.noModels}</p>
                    </div>
                  ) : (
                    sttModels.map((sttModel) => (
                      <div key={sttModel.id} className="p-4 sm:p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                        <div>
                          <div className="flex flex-wrap items-center gap-2 mb-1">
                            <span className="font-bold text-[var(--theme-on-surface)]">{sttModel.display_name}</span>
                            <span className={`px-2 py-0.5 rounded text-[10px] font-medium bg-[var(--theme-tag-bg)] text-[var(--theme-tag-text)] border border-[var(--theme-tag-border)]`}>
                              {sttModel.backend === "whisper" ? "Whisper" : sttModel.backend === "sense_voice" ? "SenseVoice" : sttModel.backend === "paraformer" ? "Paraformer" : sttModel.backend === "moonshine" ? "Moonshine" : sttModel.backend}
                            </span>
                            {sttModel.is_downloaded ? (
                              <span className="px-2 py-0.5 rounded text-[10px] font-medium bg-[var(--theme-tag-bg)] text-[var(--theme-tag-text)] border border-[var(--theme-tag-border)]">
                                {t.settings.downloaded}
                              </span>
                            ) : (
                              <span className="px-2 py-0.5 rounded text-[10px] font-medium bg-[var(--theme-input-bg)] text-[var(--theme-outline)] border border-[var(--theme-outline-variant)]">
                                {t.settings.notDownloaded}
                              </span>
                            )}
                          </div>
                          <div className="text-sm text-[var(--theme-on-surface-variant)] space-y-0.5">
                            {sttModel.description.split('\n').map((line, i) => (
                              <p key={i}>{line}</p>
                            ))}
                          </div>
                          <div className="flex items-center gap-2 mt-2">
                            <span className="text-xs text-[var(--theme-on-surface-variant)] font-medium">{sttModel.size_display}</span>
                            {sttModel.best_for_languages.length > 0 && (
                              <>
                                <span className="text-[var(--theme-outline)]">•</span>
                                <div className="flex flex-wrap gap-1">
                                  {sttModel.best_for_languages.map((lang) => (
                                    <span key={lang} className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--theme-tag-bg)] text-[var(--theme-tag-text)] border border-[var(--theme-tag-border)]">
                                      {lang}
                                    </span>
                                  ))}
                                </div>
                              </>
                            )}
                          </div>
                        </div>
                        
                        <div className="flex items-center gap-3 shrink-0">
                          {sttModel.is_active && (
                            <span className="text-sm font-medium text-[var(--theme-on-surface-variant)] border border-[var(--theme-outline-variant)] px-3 py-1.5 rounded-lg whitespace-nowrap">
                              {t.settings.active}
                            </span>
                          )}

                          {sttModel.is_downloaded && !sttModel.is_active && (
                            <button
                              type="button"
                              onClick={async () => {
                                try {
                                  await deleteModel(sttModel.id);
                                  onToast(t.settings.modelDeleted, "success");
                                } catch {
                                  onToast(t.settings.modelDeleteFailed, "error");
                                }
                              }}
                              className="text-xs text-[var(--color-error)] hover:text-[var(--theme-result-error-text)] font-medium px-2 py-1 transition-colors"
                            >
                              {t.settings.deleteModel}
                            </button>
                           )}
                           
                          {!sttModel.is_downloaded && downloadingModelId !== sttModel.id && (
                              <button
                                type="button"
                                onClick={async () => {
                                  try {
                                    await downloadModel(sttModel.id);
                                    onToast(t.settings.modelDownloaded, "success");
                                  } catch {
                                    onToast(t.settings.modelDownloadFailed, "error");
                                  }
                                }}
                                disabled={downloadingModelId !== null}
                                className="milled-button text-sm font-medium px-3 py-1.5 rounded-md whitespace-nowrap shrink-0 focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] disabled:opacity-50 disabled:cursor-not-allowed"
                              >
                              {t.settings.download}
                            </button>
                          )}
                          
                          {downloadingModelId === sttModel.id && (
                            <div className="w-24 sm:w-32 flex flex-col gap-1 text-right">
                              <div className="w-full bg-[var(--theme-progress-track)] rounded-full h-1.5 overflow-hidden">
                                <div
                                  className="bg-[var(--theme-progress-fill)] h-full rounded-full transition-all duration-300"
                                  style={{ width: `${downloadProgress}%` }}
                                />
                              </div>
                              <span className="text-[10px] text-[var(--theme-on-surface-variant)] font-medium">{downloadProgress}%</span>
                            </div>
                          )}
                          
                          {sttModel.is_downloaded && !sttModel.is_active && (
                              <button
                                type="button"
                                onClick={async () => {
                                  try {
                                    await switchModel(sttModel.id);
                                    onToast(t.settings.modelSwitched, "success");
                                  } catch {
                                    onToast(t.settings.modelSwitchFailed, "error");
                                  }
                                }}
                                disabled={downloadingModelId !== null}
                                className="text-sm font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] border border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)] px-3 py-1.5 rounded-lg focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                              >
                              {t.settings.useModel}
                            </button>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </section>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
