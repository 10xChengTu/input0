import { useEffect, useState, useCallback } from "react";
import { useSettingsStore } from "../stores/settings-store";
import { useThemeStore } from "../stores/theme-store";
import { Toast } from "../components/Toast";
import { Sidebar, type PageId } from "../components/Sidebar";
import { HomePage } from "../components/HomePage";
import { HistoryPage } from "../components/HistoryPage";
import { VocabularyPage } from "../components/VocabularyPage";
import { DataPage } from "../components/DataPage";
import { SettingsPage } from "../components/SettingsPage";
import { motion, AnimatePresence } from "framer-motion";
import { usePipelineEvents } from "../hooks/useTauriEvents";
import { useLocaleStore } from "../i18n";

export default function Settings() {
  const [activePage, setActivePage] = useState<PageId>("home");
  const [scrollToSection, setScrollToSection] = useState<string | null>(null);
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const { t } = useLocaleStore();

  useThemeStore();

  const {
    isLoading,
    loadConfig,
    checkModelStatus,
    loadModels,
    checkModelRecommendation,
  } = useSettingsStore();

  usePipelineEvents();

  useEffect(() => {
    loadConfig().then(() => {
      checkModelRecommendation(useSettingsStore.getState().language);
    });
    checkModelStatus();
    loadModels();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadConfig, checkModelStatus, loadModels, checkModelRecommendation]);

  const handleToast = (message: string, type: "success" | "error") => {
    setToast({ message, type });
  };

  const navigateToSettingsSection = useCallback((section: string) => {
    setScrollToSection(section);
    setActivePage("settings");
  }, []);

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center bg-[var(--theme-surface)]">
        <p className="text-[var(--theme-outline)]">{t.common.loading}</p>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-[var(--theme-surface)]">
      <Sidebar activePage={activePage} onNavigate={setActivePage} />

      <main className="flex-1 flex flex-col min-h-0 pt-2 pr-2 pb-2">
        <div className="flex-1 overflow-y-auto bg-[var(--theme-content-bg)] rounded-[var(--theme-content-radius)]">
          <div className="p-6">
            <AnimatePresence mode="wait">
              <motion.div
                key={activePage}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={{ duration: 0.15 }}
              >
                {activePage === "home" && <HomePage onNavigateToSettings={navigateToSettingsSection} onToast={handleToast} />}
                {activePage === "history" && <HistoryPage />}
                {activePage === "vocabulary" && <VocabularyPage onToast={handleToast} />}
                {activePage === "data" && <DataPage onToast={handleToast} />}
                {activePage === "settings" && (
                  <SettingsPage
                    onToast={handleToast}
                    scrollToSection={scrollToSection}
                    onScrollComplete={() => setScrollToSection(null)}
                  />
                )}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
      </main>

      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onDismiss={() => setToast(null)}
        />
      )}
    </div>
  );
}
