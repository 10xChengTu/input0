import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useLocaleStore } from "../i18n";
import { useVocabularyStore } from "../stores/vocabulary-store";

interface VocabularyPageProps {
  onToast: (message: string, type: "success" | "error") => void;
}

export function VocabularyPage({ onToast }: VocabularyPageProps) {
  const { t } = useLocaleStore();
  const {
    isLoading,
    isAdding,
    searchQuery,
    loadVocabulary,
    addEntry,
    removeEntry,
    setSearchQuery,
    getFilteredEntries,
  } = useVocabularyStore();

  const [term, setTerm] = useState("");

  useEffect(() => {
    loadVocabulary();
  }, [loadVocabulary]);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!term.trim()) return;

    try {
      const added = await addEntry(term.trim());
      setTerm("");
      if (added) {
        onToast(t.vocabulary.addSuccess, "success");
      } else {
        onToast(t.vocabulary.duplicate, "error");
      }
    } catch {
      onToast(t.vocabulary.addFailed, "error");
    }
  };

  const handleRemove = async (entry: string) => {
    try {
      await removeEntry(entry);
    } catch {
      onToast(t.vocabulary.removeFailed, "error");
    }
  };

  const filteredEntries = getFilteredEntries();

  return (
    <div className="flex-1 space-y-8">
      <motion.section
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.05 * 0 }}
      >
        <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider mb-4">{t.vocabulary.title}</h2>
        <p className="text-sm text-[var(--theme-on-surface-variant)]">{t.vocabulary.subtitle}</p>
      </motion.section>

      <motion.section
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.05 * 1 }}
      >
        <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden">
          <form onSubmit={handleAdd} className="p-4 sm:p-5 flex flex-col sm:flex-row items-end gap-4">
            <div className="flex-1 w-full">
              <label htmlFor="term" className="block text-sm font-medium text-[var(--theme-on-surface)] mb-1">
                {t.vocabulary.termLabel}
              </label>
              <input
                type="text"
                id="term"
                value={term}
                onChange={(e) => setTerm(e.target.value)}
                className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-2 px-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm"
                placeholder={t.vocabulary.termPlaceholder}
                disabled={isAdding}
              />
            </div>

            <div className="w-full sm:w-auto">
              <button
                type="submit"
                disabled={isAdding || !term.trim()}
                className="w-full sm:w-auto inline-flex justify-center items-center px-4 py-2 border border-[var(--theme-btn-secondary-border)] rounded-md text-sm font-medium text-[var(--theme-on-surface)] bg-[var(--theme-btn-secondary-bg)] hover:bg-[var(--theme-btn-secondary-hover-bg)] focus:outline-none focus:ring-2 focus:ring-[var(--theme-input-focus-border)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isAdding ? t.vocabulary.adding : t.vocabulary.add}
              </button>
            </div>
          </form>
        </div>
      </motion.section>

      <motion.section
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.05 * 2 }}
        className="space-y-4"
      >
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <h2 className="text-xs font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider">{t.vocabulary.entryCount(filteredEntries.length)}</h2>
          <div className="relative max-w-xs w-full sm:w-64">
            <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <svg className="h-4 w-4 text-[var(--theme-outline)]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={2} stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" />
              </svg>
            </div>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.vocabulary.searchPlaceholder}
              className="block w-full rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-1.5 pl-9 pr-3 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-1 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow sm:text-sm"
            />
          </div>
        </div>

        <div className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden divide-y divide-[var(--theme-divider)]">
          {isLoading ? (
             <div className="p-8 flex justify-center">
              <p className="text-sm text-[var(--theme-on-surface-variant)] flex items-center gap-2">
                <svg className="animate-spin h-4 w-4 text-[var(--theme-spinner)]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {t.common.loading}
              </p>
            </div>
          ) : filteredEntries.length === 0 ? (
            <div className="p-8 flex flex-col items-center justify-center text-center">
              <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-[var(--theme-surface)] mb-3">
                <svg className="h-6 w-6 text-[var(--theme-outline)]" fill="none" viewBox="0 0 24 24" strokeWidth="1.5" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25" />
                </svg>
              </div>
              <p className="text-sm font-medium text-[var(--theme-on-surface)]">{t.vocabulary.empty}</p>
              <p className="mt-1 text-xs text-[var(--theme-on-surface-variant)]">{t.vocabulary.emptyHint}</p>
            </div>
          ) : (
            <AnimatePresence initial={false}>
              {filteredEntries.map((entry) => (
                <motion.div
                  key={entry}
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: "auto" }}
                  exit={{ opacity: 0, height: 0 }}
                  transition={{ duration: 0.2 }}
                  className="p-4 sm:p-5 flex items-center justify-between gap-4"
                >
                  <span className="font-bold text-sm text-[var(--theme-primary)] bg-[var(--theme-primary)]/10 px-1.5 py-0.5 rounded">
                    {entry}
                  </span>
                  <button
                    type="button"
                    onClick={() => handleRemove(entry)}
                    className="flex-shrink-0 text-[var(--theme-outline)] hover:text-[var(--color-error)] transition-colors p-1"
                    title={t.vocabulary.remove}
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-5 h-5">
                      <path strokeLinecap="round" strokeLinejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" />
                    </svg>
                  </button>
                </motion.div>
              ))}
            </AnimatePresence>
          )}
        </div>
      </motion.section>
    </div>
  );
}
