import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { invoke } from "@tauri-apps/api/core";
import { useLocaleStore } from "../i18n";
import { useHistoryStore, type HistoryEntry } from "../stores/history-store";
import { useSettingsStore } from "../stores/settings-store";

interface ExportData {
  version: 1;
  exported_at: string;
  history: HistoryEntry[];
  vocabulary: string[];
  settings: {
    api_key: string;
    api_base_url: string;
    model: string;
    language: string;
    hotkey: string;
    text_structuring: boolean;
    user_tags: string[];
  };
}

interface DataPageProps {
  onToast: (message: string, type: "success" | "error") => void;
}

export function DataPage({ onToast }: DataPageProps) {
  const { t } = useLocaleStore();
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);
  const [pendingImport, setPendingImport] = useState<ExportData | null>(null);

  const handleExport = async () => {
    setIsExporting(true);
    try {
      const history = useHistoryStore.getState().entries;
      const vocabulary = await invoke<string[]>("get_vocabulary");
      const {
        apiKey, apiBaseUrl, model, language, hotkey,
        textStructuring, userTags,
      } = useSettingsStore.getState();

      const data: ExportData = {
        version: 1,
        exported_at: new Date().toISOString(),
        history,
        vocabulary,
        settings: {
          api_key: apiKey,
          api_base_url: apiBaseUrl,
          model,
          language,
          hotkey,
          text_structuring: textStructuring,
          user_tags: userTags,
        },
      };

      const saved = await invoke<boolean>("export_data_to_file", {
        data: JSON.stringify(data, null, 2),
      });

      if (saved) {
        onToast(t.data.exportSuccess, "success");
      }
    } catch {
      onToast(t.data.exportFailed, "error");
    } finally {
      setIsExporting(false);
    }
  };

  const handleImportClick = async () => {
    try {
      const content = await invoke<string | null>("import_data_from_file");
      if (!content) return;

      let parsed: ExportData;
      try {
        parsed = JSON.parse(content);
      } catch {
        onToast(t.data.importInvalidFormat, "error");
        return;
      }

      if (!parsed.version || !parsed.history || !parsed.vocabulary || !parsed.settings) {
        onToast(t.data.importInvalidFormat, "error");
        return;
      }

      setPendingImport(parsed);
      setShowConfirm(true);
    } catch {
      onToast(t.data.importFailed, "error");
    }
  };

  const handleImportConfirm = async () => {
    if (!pendingImport) return;
    setShowConfirm(false);
    setIsImporting(true);

    try {
      const { entries: _old, ...rest } = useHistoryStore.getState();
      void _old;
      void rest;

      localStorage.setItem("input0-history", JSON.stringify(pendingImport.history));
      useHistoryStore.setState({ entries: pendingImport.history });

      await invoke("set_vocabulary", { entries: pendingImport.vocabulary });

      const s = pendingImport.settings;
      const settingsStore = useSettingsStore.getState();
      settingsStore.setApiKey(s.api_key);
      settingsStore.setApiBaseUrl(s.api_base_url);
      settingsStore.setModel(s.model);
      settingsStore.setLanguage(s.language);
      settingsStore.setHotkey(s.hotkey);
      settingsStore.setTextStructuring(s.text_structuring);
      settingsStore.setUserTags(s.user_tags);
      await settingsStore.saveConfig();

      onToast(t.data.importSuccess, "success");
    } catch {
      onToast(t.data.importFailed, "error");
    } finally {
      setIsImporting(false);
      setPendingImport(null);
    }
  };

  const handleImportCancel = () => {
    setShowConfirm(false);
    setPendingImport(null);
  };

  return (
    <div className="space-y-4">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
      >
        <h2 className="text-xl font-semibold text-[var(--theme-on-surface)] tracking-tight">
          {t.data.title}
        </h2>
        <p className="text-xs text-[var(--theme-on-surface-variant)] mt-1">
          {t.data.subtitle}
        </p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.05 }}
        className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-5"
      >
        <div className="flex items-start justify-between">
          <div className="flex-1 mr-4">
            <div className="flex items-center gap-2 mb-1.5">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-on-surface-variant)]">
                <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3" />
              </svg>
              <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">
                {t.data.exportTitle}
              </h3>
            </div>
            <p className="text-xs text-[var(--theme-on-surface-variant)] leading-relaxed">
              {t.data.exportDescription}
            </p>
          </div>
          <button
            onClick={handleExport}
            disabled={isExporting}
            className="px-4 py-1.5 text-xs font-medium rounded-lg bg-[var(--theme-primary)] text-[var(--theme-on-primary)] hover:opacity-90 transition-opacity disabled:opacity-50 flex-shrink-0"
          >
            {isExporting ? t.data.exporting : t.data.exportButton}
          </button>
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.1 }}
        className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-5"
      >
        <div className="flex items-start justify-between">
          <div className="flex-1 mr-4">
            <div className="flex items-center gap-2 mb-1.5">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-4 h-4 text-[var(--theme-on-surface-variant)]">
                <path strokeLinecap="round" strokeLinejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" />
              </svg>
              <h3 className="text-sm font-medium text-[var(--theme-on-surface)]">
                {t.data.importTitle}
              </h3>
            </div>
            <p className="text-xs text-[var(--theme-on-surface-variant)] leading-relaxed">
              {t.data.importDescription}
            </p>
          </div>
          <button
            onClick={handleImportClick}
            disabled={isImporting}
            className="px-4 py-1.5 text-xs font-medium rounded-lg border border-[var(--theme-outline-variant)] text-[var(--theme-on-surface)] hover:bg-[var(--theme-surface-container-low)] transition-colors disabled:opacity-50 flex-shrink-0"
          >
            {isImporting ? t.data.importing : t.data.importButton}
          </button>
        </div>
      </motion.div>

      <AnimatePresence>
        {showConfirm && pendingImport && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.2 }}
            className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] p-5"
          >
            <h3 className="text-sm font-medium text-[var(--theme-on-surface)] mb-2">
              {t.data.importConfirmTitle}
            </h3>
            <p className="text-xs text-[var(--theme-on-surface-variant)] mb-4">
              {t.data.importConfirmMessage}
            </p>

            <div className="space-y-2 mb-4">
              <div className="flex items-center justify-between text-xs">
                <span className="text-[var(--theme-on-surface-variant)]">{t.data.includeHistory}</span>
                <span className="text-[var(--theme-on-surface)]">
                  {t.data.historyCount(pendingImport.history.length)}
                </span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="text-[var(--theme-on-surface-variant)]">{t.data.includeVocabulary}</span>
                <span className="text-[var(--theme-on-surface)]">
                  {t.data.vocabularyCount(pendingImport.vocabulary.length)}
                </span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="text-[var(--theme-on-surface-variant)]">{t.data.includeSettings}</span>
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3.5 h-3.5 text-[var(--theme-primary)]">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
                </svg>
              </div>
            </div>

            <div className="flex items-center gap-2 justify-end">
              <button
                onClick={handleImportCancel}
                className="px-3 py-1.5 text-xs font-medium rounded-lg border border-[var(--theme-outline-variant)] text-[var(--theme-on-surface)] hover:bg-[var(--theme-surface-container-low)] transition-colors"
              >
                {t.data.importConfirmNo}
              </button>
              <button
                onClick={handleImportConfirm}
                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--theme-primary)] text-[var(--theme-on-primary)] hover:opacity-90 transition-opacity"
              >
                {t.data.importConfirmYes}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
