import { create } from "zustand";

export type RetentionPolicy = "last_day" | "last_hour" | "last_10";

export interface HistoryEntry {
  transcribedText: string;
  optimizedText: string;
  timestamp: number;
}

interface HistoryState {
  entries: HistoryEntry[];
  retentionPolicy: RetentionPolicy;
  saveResult: (transcribedText: string, optimizedText: string) => void;
  clear: () => void;
  setRetentionPolicy: (policy: RetentionPolicy) => void;
  getFilteredEntries: () => HistoryEntry[];
}

const STORAGE_KEY = "input0-history";
const RETENTION_KEY = "input0-history-retention";

function loadEntriesFromStorage(): HistoryEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      const oldRaw = localStorage.getItem("input0-last-transcription");
      if (oldRaw) {
        const oldEntry = JSON.parse(oldRaw) as HistoryEntry;
        const entries = [oldEntry];
        localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
        localStorage.removeItem("input0-last-transcription");
        return entries;
      }
      return [];
    }
    return JSON.parse(raw) as HistoryEntry[];
  } catch {
    return [];
  }
}

function loadRetentionFromStorage(): RetentionPolicy {
  try {
    const raw = localStorage.getItem(RETENTION_KEY);
    if (raw && (raw === "last_day" || raw === "last_hour" || raw === "last_10")) {
      return raw;
    }
    return "last_10";
  } catch {
    return "last_10";
  }
}

function saveEntriesToStorage(entries: HistoryEntry[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
}

function saveRetentionToStorage(policy: RetentionPolicy) {
  localStorage.setItem(RETENTION_KEY, policy);
}

function filterEntries(entries: HistoryEntry[], policy: RetentionPolicy): HistoryEntry[] {
  const now = Date.now();
  switch (policy) {
    case "last_hour":
      return entries.filter((e) => now - e.timestamp < 60 * 60 * 1000);
    case "last_day":
      return entries.filter((e) => now - e.timestamp < 24 * 60 * 60 * 1000);
    case "last_10":
      return entries.slice(0, 10);
  }
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  entries: loadEntriesFromStorage(),
  retentionPolicy: loadRetentionFromStorage(),

  saveResult: (transcribedText: string, optimizedText: string) => {
    const entry: HistoryEntry = {
      transcribedText,
      optimizedText,
      timestamp: Date.now(),
    };
    const { entries } = get();
    const updated = [entry, ...entries].slice(0, 100);
    saveEntriesToStorage(updated);
    set({ entries: updated });
  },

  clear: () => {
    saveEntriesToStorage([]);
    set({ entries: [] });
  },

  setRetentionPolicy: (policy: RetentionPolicy) => {
    saveRetentionToStorage(policy);
    set({ retentionPolicy: policy });
  },

  getFilteredEntries: () => {
    const { entries, retentionPolicy } = get();
    return filterEntries(entries, retentionPolicy);
  },
}));
