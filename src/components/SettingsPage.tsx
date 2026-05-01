import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingsStore } from "../stores/settings-store";
import { useUpdateStore } from "../stores/update-store";
import { motion, AnimatePresence } from "framer-motion";
import { useLocaleStore } from "../i18n";
import { CustomPromptPanel } from "./CustomPromptPanel";

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

const PRESET_LLM_MODELS = [
  "gpt-5.4-nano",
  "gpt-5.4-mini",
  "gpt-5.4",
  "gpt-4o-mini",
  "gpt-4o",
] as const;

const LLM_DISABLED_VALUE = "__disabled__";
const LLM_ADD_CUSTOM_VALUE = "__add_custom__";

type SettingsTab = "general" | "api" | "models" | "custom";

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
    updateHotkey,
    userTags,
    setUserTags,
    customModels,
    setCustomModels,
    inputDevice,
    inputDevices,
    loadInputDevices,
    setInputDevice,
    hfEndpoint,
    setHfEndpoint,
  } = useSettingsStore();

  const { t } = useLocaleStore();
  const activeModel = sttModels.find((m) => m.is_active);
  const [showApiKey, setShowApiKey] = useState(false);
  const [isAddingCustomModel, setIsAddingCustomModel] = useState(false);
  const [customModelDraft, setCustomModelDraft] = useState("");
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false);
  const [accessibilityError, setAccessibilityError] = useState(false);
  const SINGLE_KEY_PRESETS = [
    "Fn",
    "RightOption", "LeftOption",
    "RightCommand", "LeftCommand",
    "RightControl", "LeftControl",
    "RightShift", "LeftShift",
  ] as const;
  const isPresetHotkey = (h: string) =>
    h === "Option+Space" || (SINGLE_KEY_PRESETS as readonly string[]).includes(h);
  const [lastCustomHotkey, setLastCustomHotkey] = useState<string>(() =>
    !isPresetHotkey(hotkey) && hotkey ? hotkey : ""
  );
  useEffect(() => {
    if (!isPresetHotkey(hotkey) && hotkey) {
      setLastCustomHotkey(hotkey);
    }
  }, [hotkey]);
  const isCustomActive = !isPresetHotkey(hotkey) && !!hotkey;
  const focusValueRef = useRef<string>("");
  const [accessibilityGranted, setAccessibilityGranted] = useState<boolean | null>(null);
  const [micPermission, setMicPermission] = useState<string | null>(null);

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

  useEffect(() => {
    invoke<boolean>("check_accessibility_permission").then(setAccessibilityGranted).catch(() => {});
    invoke<string>("check_microphone_permission").then(setMicPermission).catch(() => {});
    loadInputDevices();

    const recheckPermissions = () => {
      invoke<boolean>("check_accessibility_permission").then(setAccessibilityGranted).catch(() => {});
      invoke<string>("check_microphone_permission").then(setMicPermission).catch(() => {});
    };

    // Use Tauri native window focus event — more reliable than DOM window.focus
    // when the user switches back from macOS System Settings.
    let unlistenFn: (() => void) | null = null;
    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) recheckPermissions();
    }).then((fn) => { unlistenFn = fn; });

    return () => { unlistenFn?.(); };
  }, [loadInputDevices]);

  const sectionToTab = (section: string | null | undefined): SettingsTab => {
    if (section === "stt-model") return "models";
    if (section === "hotkey" || section === "language") return "general";
    if (section === "api-key") return "api";
    if (section === "user-tags") return "general";
    if (section === "update") return "general";
    return "general";
  };

  const [activeTab, setActiveTab] = useState<SettingsTab>(
    sectionToTab(scrollToSection)
  );

  useEffect(() => {
    if (scrollToSection) {
      setActiveTab(sectionToTab(scrollToSection));
      // Delay scroll to allow tab content to render
      requestAnimationFrame(() => {
        const el = document.getElementById(`section-${scrollToSection}`);
        if (el) {
          el.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      });
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

  const tabs: { id: SettingsTab; label: string; beta?: boolean }[] = [
    { id: "general", label: t.settings.tabGeneral },
    { id: "api", label: t.settings.tabApi },
    { id: "models", label: t.settings.tabModels },
    { id: "custom", label: t.settings.tabCustom, beta: true },
  ];

  return (
    <div className="flex-1 flex flex-col min-h-0">
      <div className="inline-flex self-start gap-1 p-1 mb-6 bg-[var(--theme-surface-container)] rounded-xl">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveTab(tab.id)}
            className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-[10px] text-sm font-medium transition-all ${
              activeTab === tab.id
                ? "bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)] shadow-sm"
                : "text-[var(--theme-on-surface-variant)] hover:text-[var(--theme-on-surface)]"
            }`}
          >
            {tab.label}
            {tab.beta && (
              <span
                className="px-1 py-px text-[9px] font-semibold uppercase tracking-wider rounded-sm leading-none"
                style={{ backgroundColor: "var(--theme-primary)", color: "white" }}
              >
                Beta
              </span>
            )}
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

                  <div className="p-4 sm:p-5">
                    {accessibilityError && (
                      <div
                        className="mb-4 rounded-md border p-3"
                        style={{
                          backgroundColor: "var(--theme-warning-bg)",
                          borderColor: "var(--theme-warning-border)",
                        }}
                      >
                        <p
                          className="text-sm font-medium"
                          style={{ color: "var(--theme-warning-text)" }}
                        >
                          {t.settings.hotkeyPermissionBannerTitle}
                        </p>
                        <p
                          className="mt-1 text-xs"
                          style={{ color: "var(--theme-warning-text)" }}
                        >
                          {t.settings.hotkeyPermissionBannerBody}
                        </p>
                        <button
                          type="button"
                          onClick={async () => {
                            await invoke("open_accessibility_settings");
                            setAccessibilityError(false);
                          }}
                          className="mt-2 rounded-md bg-[var(--theme-primary)] px-3 py-1 text-xs font-medium text-white hover:bg-[var(--theme-primary-hover)] transition-colors"
                        >
                          {t.settings.hotkeyPermissionBannerAction}
                        </button>
                      </div>
                    )}
                    <div className="flex items-center justify-between">
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
                            } catch (err) {
                              const message = String(err ?? "");
                              if (message.toLowerCase().includes("accessibility")) {
                                setAccessibilityError(true);
                              }
                              onToast(t.settings.hotkeyChangeFailed, "error");
                            }
                          }}
                          onCancel={() => setIsRecordingHotkey(false)}
                          recordingText={t.settings.hotkeyRecording}
                        />
                      ) : (
                        <>
                          {isCustomActive && (
                            <button
                              type="button"
                              onClick={() => setIsRecordingHotkey(true)}
                              className="text-xs font-medium text-[var(--theme-primary)] hover:text-[var(--theme-primary-hover)] transition-colors"
                            >
                              {t.settings.hotkeyChange}
                            </button>
                          )}
                          <select
                            value={isCustomActive ? "__custom__" : hotkey}
                            onChange={async (e) => {
                              const value = e.target.value;
                              if (value === "__custom__") {
                                if (lastCustomHotkey) {
                                  if (lastCustomHotkey === hotkey) return;
                                  try {
                                    await updateHotkey(lastCustomHotkey);
                                    onToast(t.settings.hotkeyChanged, "success");
                                  } catch (err) {
                                    const message = String(err ?? "");
                                    if (message.toLowerCase().includes("accessibility")) {
                                      setAccessibilityError(true);
                                    }
                                    onToast(t.settings.hotkeyChangeFailed, "error");
                                  }
                                } else {
                                  setIsRecordingHotkey(true);
                                }
                                return;
                              }
                              if (value === hotkey) return;
                              try {
                                await updateHotkey(value);
                                onToast(t.settings.hotkeyChanged, "success");
                              } catch (err) {
                                const message = String(err ?? "");
                                if (message.toLowerCase().includes("accessibility")) {
                                  setAccessibilityError(true);
                                }
                                onToast(t.settings.hotkeyChangeFailed, "error");
                              }
                            }}
                            className="rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-1.5 pl-3 pr-8 text-sm text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow"
                          >
                            <optgroup label={t.settings.hotkeyComboGroup}>
                              <option value="Option+Space" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetOptionSpace}</option>
                            </optgroup>
                            <optgroup label={t.settings.hotkeySingleKeyGroup}>
                              <option value="Fn" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetFn}</option>
                              <option value="RightOption" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetRightOption}</option>
                              <option value="LeftOption" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetLeftOption}</option>
                              <option value="RightCommand" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetRightCommand}</option>
                              <option value="LeftCommand" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetLeftCommand}</option>
                              <option value="RightControl" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetRightControl}</option>
                              <option value="LeftControl" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetLeftControl}</option>
                              <option value="RightShift" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetRightShift}</option>
                              <option value="LeftShift" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.hotkeyPresetLeftShift}</option>
                            </optgroup>
                            <option value="__custom__" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">
                              {lastCustomHotkey
                                ? `${t.settings.hotkeyPresetCustom}: ${lastCustomHotkey}`
                                : `${t.settings.hotkeyPresetCustom}…`}
                            </option>
                          </select>
                        </>
                      )}
                    </div>
                    </div>
                    {hotkey === "RightOption" && (
                      <p className="mt-2 text-xs text-[var(--theme-warning-text)]">
                        {t.settings.hotkeySingleKeyWarningRightOption}
                      </p>
                    )}
                    {(hotkey === "RightCommand" || hotkey === "LeftCommand") && (
                      <p className="mt-2 text-xs text-[var(--theme-warning-text)]">
                        {t.settings.hotkeySingleKeyWarningCommand}
                      </p>
                    )}
                    {hotkey === "Fn" && (
                      <p className="mt-2 text-xs text-[var(--theme-warning-text)]">
                        {t.settings.hotkeySingleKeyWarningFn}
                      </p>
                    )}
                  </div>

                </div>
              </section>

              <section>
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.permissionsTitle}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
                  <div className="p-4 sm:p-5 flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.accessibilityLabel}</h3>
                      <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.accessibilityHint}</p>
                    </div>
                    <div className="ml-4 flex-shrink-0 flex items-center gap-3">
                      {accessibilityGranted !== null && (
                        <span className={`text-xs font-medium ${accessibilityGranted ? "text-[var(--theme-status-dot-loaded)]" : "text-[var(--color-error)]"}`}>
                          {accessibilityGranted ? t.settings.accessibilityGranted : t.settings.accessibilityNotGranted}
                        </span>
                      )}
                      {accessibilityGranted === false && (
                        <>
                          <button
                            type="button"
                            onClick={async () => {
                              const granted = await invoke<boolean>("request_accessibility_permission");
                              setAccessibilityGranted(granted);
                              if (!granted) {
                                let attempts = 0;
                                const poll = setInterval(() => {
                                  attempts++;
                                  invoke<boolean>("check_accessibility_permission").then((status) => {
                                    setAccessibilityGranted(status);
                                    if (status || attempts >= 15) {
                                      clearInterval(poll);
                                    }
                                  }).catch(() => {
                                    clearInterval(poll);
                                  });
                                }, 1000);
                              }
                            }}
                            className="milled-button text-xs font-medium px-3 py-1.5 rounded-lg whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)]"
                          >
                            {t.settings.accessibilityGrant}
                          </button>
                          <button
                            type="button"
                            onClick={() => invoke("open_accessibility_settings")}
                            className="text-xs font-medium text-[var(--theme-primary)] hover:text-[var(--theme-primary-hover)] transition-colors whitespace-nowrap"
                          >
                            {t.settings.accessibilityOpenSettings}
                          </button>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="p-4 sm:p-5 flex items-center justify-between">
                    <div>
                      <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">{t.settings.microphoneLabel}</h3>
                      <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.settings.microphoneHint}</p>
                    </div>
                    <div className="ml-4 flex-shrink-0 flex items-center gap-3">
                      {micPermission !== null && (
                        <span className={`text-xs font-medium ${micPermission === "authorized" ? "text-[var(--theme-status-dot-loaded)]" : "text-[var(--color-error)]"}`}>
                          {micPermission === "authorized" ? t.settings.microphoneGranted : micPermission === "not_determined" ? t.settings.microphoneNotDetermined : t.settings.microphoneNotGranted}
                        </span>
                      )}
                      {micPermission !== null && micPermission !== "authorized" && (
                        <>
                          {micPermission === "not_determined" && (
                            <button
                              type="button"
                              onClick={() => {
                                invoke("request_microphone_permission");
                                // Poll permission status after macOS system dialog / settings
                                let attempts = 0;
                                const poll = setInterval(() => {
                                  attempts++;
                                  invoke<string>("check_microphone_permission").then((status) => {
                                    setMicPermission(status);
                                    if (status !== "not_determined" || attempts >= 15) {
                                      clearInterval(poll);
                                    }
                                  }).catch(() => {
                                    clearInterval(poll);
                                  });
                                }, 1000);
                              }}
                              className="milled-button text-xs font-medium px-3 py-1.5 rounded-lg whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)]"
                            >
                              {t.settings.microphoneGrant}
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => invoke("open_microphone_settings")}
                            className="text-xs font-medium text-[var(--theme-primary)] hover:text-[var(--theme-primary-hover)] transition-colors whitespace-nowrap"
                          >
                            {t.settings.microphoneOpenSettings}
                          </button>
                        </>
                      )}
                    </div>
                  </div>

                  <div className="p-4 sm:p-5">
                    <label htmlFor="inputDevice" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.inputDeviceLabel}
                    </label>
                    <select
                      id="inputDevice"
                      value={inputDevice}
                      onChange={async (e) => {
                        try {
                          await setInputDevice(e.target.value);
                          onToast(t.settings.settingsSaved, "success");
                        } catch {
                          onToast(t.settings.settingsSaveFailed, "error");
                        }
                      }}
                      className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 pl-3 pr-10 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                    >
                      <option value="" className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">{t.settings.inputDeviceDefault}</option>
                      {inputDevices.map((device) => (
                        <option key={device.name} value={device.name} className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">
                          {device.name}{device.is_default ? ` (${t.settings.inputDeviceDefault})` : ""}
                        </option>
                      ))}
                    </select>
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {t.settings.inputDeviceHint}
                    </p>
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

              <section id="section-update">
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
                    <div className="flex gap-2">
                      <select
                        id="model"
                        value={model === "" ? LLM_DISABLED_VALUE : model}
                        onChange={async (e) => {
                          const v = e.target.value;
                          if (v === LLM_ADD_CUSTOM_VALUE) {
                            setCustomModelDraft("");
                            setIsAddingCustomModel(true);
                            return;
                          }
                          const next = v === LLM_DISABLED_VALUE ? "" : v;
                          setModel(next);
                          try {
                            await saveField("model", next);
                            onToast(t.settings.settingsSaved, "success");
                          } catch {
                            onToast(t.settings.settingsSaveFailed, "error");
                          }
                        }}
                        className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                      >
                        <option value={LLM_DISABLED_VALUE}>{t.settings.modelDisabledOption}</option>
                        <optgroup label={t.settings.modelPresetGroup}>
                          {PRESET_LLM_MODELS.map((m) => (
                            <option key={m} value={m}>{m}</option>
                          ))}
                        </optgroup>
                        {customModels.length > 0 && (
                          <optgroup label={t.settings.modelCustomGroup}>
                            {customModels.map((m) => (
                              <option key={m} value={m}>{m}</option>
                            ))}
                          </optgroup>
                        )}
                        {model &&
                          model !== "" &&
                          !PRESET_LLM_MODELS.includes(model as typeof PRESET_LLM_MODELS[number]) &&
                          !customModels.includes(model) && (
                            <option value={model}>{model}</option>
                          )}
                        <option value={LLM_ADD_CUSTOM_VALUE}>＋ {t.settings.modelAddCustom}</option>
                      </select>
                      {customModels.includes(model) && (
                        <button
                          type="button"
                          title={t.settings.modelRemoveCustom}
                          onClick={async () => {
                            const next = customModels.filter((m) => m !== model);
                            try {
                              await setCustomModels(next);
                              setModel("");
                              await saveField("model", "");
                              onToast(t.settings.settingsSaved, "success");
                            } catch {
                              onToast(t.settings.settingsSaveFailed, "error");
                            }
                          }}
                          className="shrink-0 inline-flex items-center text-sm font-medium text-[var(--color-error)] hover:text-[var(--theme-result-error-text)] px-2 py-2 transition-colors"
                        >
                          {t.settings.modelRemoveCustomLabel}
                        </button>
                      )}
                    </div>
                    {isAddingCustomModel && (() => {
                      const trimmedDraft = customModelDraft.trim();
                      const confirmAdd = async () => {
                        if (!trimmedDraft) return;
                        if (
                          !PRESET_LLM_MODELS.includes(trimmedDraft as typeof PRESET_LLM_MODELS[number]) &&
                          !customModels.includes(trimmedDraft)
                        ) {
                          try {
                            await setCustomModels([...customModels, trimmedDraft]);
                          } catch {
                            onToast(t.settings.settingsSaveFailed, "error");
                            return;
                          }
                        }
                        setModel(trimmedDraft);
                        try {
                          await saveField("model", trimmedDraft);
                          onToast(t.settings.settingsSaved, "success");
                        } catch {
                          onToast(t.settings.settingsSaveFailed, "error");
                        }
                        setIsAddingCustomModel(false);
                        setCustomModelDraft("");
                      };
                      const cancelAdd = () => {
                        setIsAddingCustomModel(false);
                        setCustomModelDraft("");
                      };
                      return (
                        <div className="mt-2 flex gap-2">
                          <input
                            type="text"
                            autoFocus
                            value={customModelDraft}
                            placeholder={t.settings.modelAddCustomPrompt}
                            onChange={(e) => setCustomModelDraft(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Enter") {
                                e.preventDefault();
                                confirmAdd();
                              } else if (e.key === "Escape") {
                                e.preventDefault();
                                cancelAdd();
                              }
                            }}
                            className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                          />
                          <button
                            type="button"
                            onClick={confirmAdd}
                            disabled={!trimmedDraft}
                            className="shrink-0 inline-flex items-center px-4 py-2 border border-[var(--theme-btn-secondary-border)] rounded-md text-sm font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                          >
                            {t.settings.modelAddCustomConfirm}
                          </button>
                          <button
                            type="button"
                            onClick={cancelAdd}
                            className="shrink-0 inline-flex items-center px-4 py-2 border border-[var(--theme-btn-secondary-border)] rounded-md text-sm font-medium text-[var(--theme-on-surface-variant)] bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors"
                          >
                            {t.settings.modelAddCustomCancel}
                          </button>
                        </div>
                      );
                    })()}
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {model === "" ? t.settings.modelHintDisabled : t.settings.modelHint}
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
                        disabled={isTesting || !apiKey || !model}
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
                <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.settings.hfEndpointLabel}</h2>
                <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden">
                  <div className="p-4 sm:p-5">
                    <label htmlFor="hfEndpoint" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                      {t.settings.hfEndpointLabel}
                    </label>
                    <div className="flex items-center gap-2">
                      <input
                        type="text"
                        id="hfEndpoint"
                        value={hfEndpoint}
                        onChange={(e) => setHfEndpoint(e.target.value)}
                        onFocus={() => handleFocusCapture(hfEndpoint)}
                        onBlur={() => handleBlurSave("hf_endpoint", hfEndpoint)}
                        className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm sm:leading-6"
                        placeholder={t.settings.hfEndpointPlaceholder}
                      />
                      <button
                        type="button"
                        onClick={async () => {
                          setHfEndpoint("https://hf-mirror.com");
                          try {
                            await saveField("hf_endpoint", "https://hf-mirror.com");
                            onToast(t.settings.settingsSaved, "success");
                          } catch {
                            onToast(t.settings.settingsSaveFailed, "error");
                          }
                        }}
                        className={`flex-shrink-0 px-3 py-2 rounded-md text-xs font-medium border transition-colors whitespace-nowrap ${
                          hfEndpoint === "https://hf-mirror.com"
                            ? "bg-[var(--theme-primary)] text-[var(--theme-on-primary)] border-[var(--theme-primary)]"
                            : "bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)] border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)]"
                        }`}
                      >
                        hf-mirror.com
                      </button>
                      {hfEndpoint !== "https://huggingface.co" && (
                        <button
                          type="button"
                          onClick={async () => {
                            setHfEndpoint("https://huggingface.co");
                            try {
                              await saveField("hf_endpoint", "https://huggingface.co");
                              onToast(t.settings.settingsSaved, "success");
                            } catch {
                              onToast(t.settings.settingsSaveFailed, "error");
                            }
                          }}
                          className="flex-shrink-0 px-3 py-2 rounded-md text-xs font-medium border bg-[var(--theme-btn-secondary-bg)] text-[var(--theme-on-surface)] border-[var(--theme-btn-secondary-border)] hover:bg-[var(--theme-btn-secondary-hover-bg)] transition-colors whitespace-nowrap"
                        >
                          Reset
                        </button>
                      )}
                    </div>
                    <p className="mt-2 text-xs text-[var(--theme-on-surface-variant)]">
                      {t.settings.hfEndpointHint}
                    </p>
                  </div>
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
          {activeTab === "custom" && (
            <motion.div
              key="custom"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="space-y-8"
            >
              <CustomPromptPanel onToast={onToast} />
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
