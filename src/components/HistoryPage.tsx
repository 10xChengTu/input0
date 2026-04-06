import { useState, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useHistoryStore, type RetentionPolicy } from "../stores/history-store";
import { useLocaleStore } from "../i18n";

export function HistoryPage() {
  const { entries, retentionPolicy, setRetentionPolicy, getFilteredEntries, clear } = useHistoryStore();
  const { t, locale } = useLocaleStore();
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [copyError, setCopyError] = useState(false);

  const filteredEntries = getFilteredEntries();

  const retentionOptions: { value: RetentionPolicy; label: string }[] = [
    { value: "last_10", label: t.history.retentionLast10 },
    { value: "last_hour", label: t.history.retentionLastHour },
    { value: "last_day", label: t.history.retentionLastDay },
  ];

  const handleCopy = useCallback(async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setCopyError(false);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      setCopyError(true);
      setTimeout(() => setCopyError(false), 2000);
    }
  }, []);

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();
    const localeStr = locale === "zh" ? "zh-CN" : "en-US";
    const timeStr = date.toLocaleTimeString(localeStr, { hour: "2-digit", minute: "2-digit" });
    if (isToday) return `${t.history.todayPrefix} ${timeStr}`;
    return `${date.toLocaleDateString(localeStr, { month: "short", day: "numeric" })} ${timeStr}`;
  };

  if (entries.length === 0) {
    return (
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="flex flex-col items-center justify-center h-full min-h-[400px] text-center"
      >
        <div className="w-12 h-12 rounded-full bg-[var(--theme-surface-container-lowest)] border border-[var(--theme-outline-variant)] flex items-center justify-center mb-4">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-6 h-6 text-[var(--theme-outline)]">
            <path strokeLinecap="round" strokeLinejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
          </svg>
        </div>
        <h3 className="text-sm font-medium text-[var(--theme-on-surface)] mb-1">{t.history.noRecords}</h3>
        <p className="text-xs text-[var(--theme-on-surface-variant)] max-w-[200px]">
          {t.history.noRecordsHint}
        </p>
      </motion.div>
    );
  }

  return (
    <div className="space-y-4">
      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="flex items-center justify-between"
      >
        <div>
          <h2 className="text-xl font-semibold text-[var(--theme-on-surface)] tracking-tight">{t.history.title}</h2>
          <p className="text-xs text-[var(--theme-on-surface-variant)] mt-1">
            {filteredEntries.length === 0 ? t.history.noFilteredRecords : t.history.recordCount(filteredEntries.length)}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={retentionPolicy}
            onChange={(e) => setRetentionPolicy(e.target.value as RetentionPolicy)}
            className="text-xs rounded-md border border-[var(--theme-outline-variant)] bg-[var(--theme-input-bg)] py-1 pl-2 pr-6 text-[var(--theme-on-surface)] focus:border-[var(--theme-input-focus-border)] focus:ring-2 focus:ring-[var(--theme-input-focus-border)] outline-none transition-shadow"
          >
            {retentionOptions.map((opt) => (
              <option key={opt.value} value={opt.value} className="bg-[var(--theme-surface)] text-[var(--theme-on-surface)]">
                {opt.label}
              </option>
            ))}
          </select>
          <button
            onClick={clear}
            className="text-xs text-[var(--theme-outline)] hover:text-[var(--theme-on-surface-variant)] transition-colors px-2 py-1"
          >
            {t.history.clearAll}
          </button>
        </div>
      </motion.div>

      <div className="space-y-3">
        <AnimatePresence initial={false}>
          {filteredEntries.map((entry, index) => {
            const entryKey = `${entry.timestamp}-${index}`;
            return (
              <motion.div
                key={entryKey}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -8 }}
                transition={{ duration: 0.2, delay: index * 0.03 }}
                className="bg-[var(--theme-surface-container-lowest)] rounded-xl border border-[var(--theme-outline-variant)] overflow-hidden"
              >
                <div className="px-4 py-2.5 flex items-center justify-between border-b border-[var(--theme-divider)]">
                  <span className="text-[11px] text-[var(--theme-on-surface-variant)]">
                    {formatTime(entry.timestamp)}
                  </span>
                </div>

                <div className="divide-y divide-[var(--theme-divider)]">
                  <div className="p-4">
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="text-[11px] font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider">{t.history.transcribed}</span>
                      <button
                        onClick={() => handleCopy(entry.transcribedText, `${entryKey}-t`)}
                        className="text-[11px] text-[var(--theme-outline)] hover:text-[var(--theme-on-surface-variant)] transition-colors flex items-center gap-1"
                      >
                        {copiedId === `${entryKey}-t` ? (
                          <>
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3 h-3">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
                            </svg>
                            {t.history.copied}
                          </>
                        ) : (
                          <>
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3 h-3">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M15.666 3.888A2.25 2.25 0 0013.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 01-.75.75H9.75a.75.75 0 01-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 01-2.25 2.25H6.75A2.25 2.25 0 014.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 011.927-.184" />
                            </svg>
                            {t.history.copy}
                          </>
                        )}
                      </button>
                    </div>
                    <p className="text-sm text-[var(--theme-on-surface)] leading-relaxed whitespace-pre-wrap line-clamp-3">
                      {entry.transcribedText}
                    </p>
                  </div>

                  <div className="p-4">
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="text-[11px] font-semibold text-[var(--theme-on-surface-variant)] uppercase tracking-wider">{t.history.optimized}</span>
                      <button
                        onClick={() => handleCopy(entry.optimizedText, `${entryKey}-o`)}
                        className="text-[11px] text-[var(--theme-outline)] hover:text-[var(--theme-on-surface-variant)] transition-colors flex items-center gap-1"
                      >
                        {copiedId === `${entryKey}-o` ? (
                          <>
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3 h-3">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M4.5 12.75l6 6 9-13.5" />
                            </svg>
                            {t.history.copied}
                          </>
                        ) : (
                          <>
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" strokeWidth={1.5} stroke="currentColor" className="w-3 h-3">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M15.666 3.888A2.25 2.25 0 0013.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 01-.75.75H9.75a.75.75 0 01-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 01-2.25 2.25H6.75A2.25 2.25 0 014.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 011.927-.184" />
                            </svg>
                            {t.history.copy}
                          </>
                        )}
                      </button>
                    </div>
                    <p className="text-sm text-[var(--theme-on-surface)] leading-relaxed whitespace-pre-wrap line-clamp-3">
                      {entry.optimizedText}
                    </p>
                  </div>
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>

      <AnimatePresence>
        {copyError && (
          <motion.p
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="text-xs text-[var(--theme-result-error-text)] mt-2"
          >
            {t.history.copyFailed}
          </motion.p>
        )}
      </AnimatePresence>
    </div>
  );
}
