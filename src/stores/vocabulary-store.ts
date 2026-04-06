import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface VocabularyState {
  entries: string[];
  isLoading: boolean;
  isAdding: boolean;
  searchQuery: string;
  loadVocabulary: () => Promise<void>;
  addEntry: (term: string) => Promise<boolean>;
  removeEntry: (term: string) => Promise<void>;
  validateAndAdd: (original: string, correct: string) => Promise<boolean>;
  setSearchQuery: (query: string) => void;
  getFilteredEntries: () => string[];
}

export const useVocabularyStore = create<VocabularyState>((set, get) => ({
  entries: [],
  isLoading: false,
  isAdding: false,
  searchQuery: "",

  loadVocabulary: async () => {
    set({ isLoading: true });
    try {
      const entries = await invoke<string[]>("get_vocabulary");
      set({ entries, isLoading: false });
    } catch {
      set({ isLoading: false });
    }
  },

  addEntry: async (term: string) => {
    set({ isAdding: true });
    try {
      const added = await invoke<boolean>("add_vocabulary_entry", { term });
      const entries = await invoke<string[]>("get_vocabulary");
      set({ entries, isAdding: false });
      return added;
    } catch (e) {
      set({ isAdding: false });
      throw e;
    }
  },

  removeEntry: async (term: string) => {
    try {
      await invoke("remove_vocabulary_entry", { term });
      const entries = await invoke<string[]>("get_vocabulary");
      set({ entries });
    } catch (e) {
      throw e;
    }
  },

  validateAndAdd: async (original: string, correct: string) => {
    set({ isAdding: true });
    try {
      const result = await invoke<boolean>("validate_and_add_vocabulary", { original, correct });
      if (result) {
        const entries = await invoke<string[]>("get_vocabulary");
        set({ entries });
      }
      set({ isAdding: false });
      return result;
    } catch (e) {
      set({ isAdding: false });
      throw e;
    }
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
  },

  getFilteredEntries: () => {
    const { entries, searchQuery } = get();
    if (!searchQuery.trim()) return entries;
    const q = searchQuery.toLowerCase();
    return entries.filter((term) => term.toLowerCase().includes(q));
  },
}));
