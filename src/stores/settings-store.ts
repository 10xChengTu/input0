import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AudioDeviceInfo {
  name: string;
  is_default: boolean;
}

export interface SttModelInfo {
  id: string;
  display_name: string;
  description: string;
  backend: "whisper" | "sense_voice" | "paraformer" | "moonshine";
  total_size_bytes: number;
  size_display: string;
  best_for_languages: string[];
  is_downloaded: boolean;
  is_active: boolean;
}

export interface RecommendedModel {
  id: string;
  name: string;
  reason: string;
}

export interface ModelRecommendation {
  should_switch: boolean;
  recommended_models: RecommendedModel[];
  current_model_id: string;
}

interface DownloadProgress {
  model_id: string;
  file_name: string;
  downloaded_bytes: number;
  total_bytes: number;
  file_index: number;
  total_files: number;
}

export interface SettingsState {
  apiKey: string;
  apiBaseUrl: string;
  model: string;
  language: string;
  hotkey: string;
  modelPath: string;
  isModelLoaded: boolean;
  isSaving: boolean;
  isLoading: boolean;
  isTesting: boolean;
  testResult: { success: boolean; message: string } | null;

  sttModels: SttModelInfo[];
  isLoadingModels: boolean;
  downloadingModelId: string | null;
  downloadProgress: number;
  modelRecommendation: ModelRecommendation | null;

  textStructuring: boolean;

  userTags: string[];

  customModels: string[];

  inputDevice: string;
  inputDevices: AudioDeviceInfo[];

  onboardingCompleted: boolean;

  hfEndpoint: string;
  customPromptEnabled: boolean;
  customPrompt: string;
  structuringPrompt: string;

  setApiKey: (key: string) => void;
  setApiBaseUrl: (url: string) => void;
  setModel: (model: string) => void;
  setLanguage: (lang: string) => void;
  setHotkey: (hotkey: string) => void;
  setTextStructuring: (enabled: boolean) => void;
  setUserTags: (tags: string[]) => void;
  setCustomModels: (models: string[]) => Promise<void>;
  setHfEndpoint: (endpoint: string) => void;
  setCustomPromptEnabled: (enabled: boolean) => void;
  setCustomPrompt: (prompt: string) => void;
  setStructuringPrompt: (prompt: string) => void;
  setOnboardingCompleted: (completed: boolean) => void;
  loadInputDevices: () => Promise<void>;
  setInputDevice: (deviceName: string) => Promise<void>;
  completeOnboarding: () => Promise<void>;
  loadConfig: () => Promise<void>;
  saveConfig: () => Promise<void>;
  saveField: (field: string, value: string) => Promise<void>;
  updateHotkey: (hotkey: string) => Promise<void>;
  checkModelStatus: () => Promise<void>;
  testApiConnection: () => Promise<void>;

  loadModels: () => Promise<void>;
  downloadModel: (modelId: string) => Promise<void>;
  switchModel: (modelId: string) => Promise<void>;
  deleteModel: (modelId: string) => Promise<void>;
  checkModelRecommendation: (language: string) => Promise<void>;
}

interface AppConfig {
  api_key: string;
  api_base_url: string;
  model: string;
  language: string;
  hotkey: string;
  model_path: string;
  text_structuring: boolean;
  user_tags: string[];
  custom_models: string[];
  onboarding_completed: boolean;
  input_device: string;
  hf_endpoint: string;
  custom_prompt_enabled: boolean;
  custom_prompt: string;
  structuring_prompt: string;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  apiKey: "",
  apiBaseUrl: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  language: "auto",
  hotkey: "Option+Space",
  modelPath: "",
  isModelLoaded: false,
  isSaving: false,
  isLoading: true,
  isTesting: false,
  testResult: null,

  sttModels: [],
  isLoadingModels: false,
  downloadingModelId: null,
  downloadProgress: 0,
  modelRecommendation: null,
  textStructuring: false,
  userTags: [],
  customModels: [],
  inputDevice: "",
  inputDevices: [],
  onboardingCompleted: false,
  hfEndpoint: "https://huggingface.co",
  customPromptEnabled: false,
  customPrompt: "",
  structuringPrompt: "",

  setApiKey: (apiKey) => set({ apiKey }),
  setApiBaseUrl: (apiBaseUrl) => set({ apiBaseUrl }),
  setModel: (model) => set({ model }),
  setLanguage: (language) => set({ language }),
  setHotkey: (hotkey) => set({ hotkey }),
  setTextStructuring: (textStructuring) => set({ textStructuring }),
  setUserTags: (userTags) => set({ userTags }),
  setCustomModels: async (customModels) => {
    const prev = get().customModels;
    set({ customModels });
    try {
      await invoke("update_config_field", {
        field: "custom_models",
        value: JSON.stringify(customModels),
      });
    } catch (error) {
      console.error("Failed to save custom_models:", error);
      set({ customModels: prev });
      throw error;
    }
  },
  setHfEndpoint: (hfEndpoint) => set({ hfEndpoint }),
  setCustomPromptEnabled: (customPromptEnabled) => set({ customPromptEnabled }),
  setCustomPrompt: (customPrompt) => set({ customPrompt }),
  setStructuringPrompt: (structuringPrompt) => set({ structuringPrompt }),
  setOnboardingCompleted: (onboardingCompleted) => set({ onboardingCompleted }),
  completeOnboarding: async () => {
    set({ onboardingCompleted: true });
    try {
      await invoke("update_config_field", { field: "onboarding_completed", value: "true" });
    } catch (error) {
      console.error("Failed to save onboarding_completed:", error);
      set({ onboardingCompleted: false });
    }
  },

  loadInputDevices: async () => {
    try {
      const devices = await invoke<AudioDeviceInfo[]>("list_input_devices");
      set({ inputDevices: devices });
    } catch (error) {
      console.error("Failed to load input devices:", error);
    }
  },

  setInputDevice: async (deviceName: string) => {
    const prev = get().inputDevice;
    set({ inputDevice: deviceName });
    try {
      await invoke("set_input_device", { deviceName });
    } catch (error) {
      console.error("Failed to set input device:", error);
      set({ inputDevice: prev });
      throw error;
    }
  },
  
  loadConfig: async () => {
    set({ isLoading: true });
    try {
      const config = await invoke<AppConfig>("get_config");
      set({
        apiKey: config.api_key || "",
        apiBaseUrl: config.api_base_url || "https://api.openai.com/v1",
        model: config.model || "gpt-4o-mini",
        language: config.language === "zh" ? "zh-CN" : (config.language || "auto"),
        hotkey: config.hotkey || "Option+Space",
        modelPath: config.model_path || "",
        textStructuring: config.text_structuring ?? true,
        userTags: config.user_tags ?? [],
        customModels: config.custom_models ?? [],
        onboardingCompleted: config.onboarding_completed ?? false,
        inputDevice: config.input_device || "",
        hfEndpoint: config.hf_endpoint || "https://huggingface.co",
        customPromptEnabled: config.custom_prompt_enabled ?? false,
        customPrompt: config.custom_prompt ?? "",
        structuringPrompt: config.structuring_prompt ?? "",
      });
    } catch (error) {
      console.error("Failed to load config:", error);
    } finally {
      set({ isLoading: false });
    }
  },

  saveConfig: async () => {
    set({ isSaving: true });
    try {
      const state = get();
      const config: AppConfig = {
        api_key: state.apiKey,
        api_base_url: state.apiBaseUrl,
        model: state.model,
        language: state.language,
        hotkey: state.hotkey,
        model_path: state.modelPath,
        text_structuring: state.textStructuring,
        user_tags: state.userTags,
        custom_models: state.customModels,
        onboarding_completed: state.onboardingCompleted,
        input_device: state.inputDevice,
        hf_endpoint: state.hfEndpoint,
        custom_prompt_enabled: state.customPromptEnabled,
        custom_prompt: state.customPrompt,
        structuring_prompt: state.structuringPrompt,
      };
      await invoke("save_config", { config });
    } catch (error) {
      console.error("Failed to save config:", error);
      throw error;
    } finally {
      set({ isSaving: false });
    }
  },

  saveField: async (field: string, value: string) => {
    try {
      await invoke("update_config_field", { field, value });
    } catch (error) {
      console.error(`Failed to save field ${field}:`, error);
      throw error;
    }
  },

  updateHotkey: async (hotkeyStr: string) => {
    await invoke("update_hotkey", { hotkeyStr });
    set({ hotkey: hotkeyStr });
  },
  
  checkModelStatus: async () => {
    try {
      const isLoaded = await invoke<boolean>("is_whisper_model_loaded");
      set({ isModelLoaded: isLoaded });
    } catch (error) {
      console.error("Failed to check model status:", error);
      set({ isModelLoaded: false });
    }
  },
  
  testApiConnection: async () => {
    set({ isTesting: true, testResult: null });
    try {
      const state = get();
      const message = await invoke<string>("test_api_connection", {
        apiKey: state.apiKey,
        baseUrl: state.apiBaseUrl,
        model: state.model,
      });
      set({ testResult: { success: true, message } });
    } catch (error) {
      set({ testResult: { success: false, message: String(error) } });
    } finally {
      set({ isTesting: false });
    }
  },

  loadModels: async () => {
    set({ isLoadingModels: true });
    try {
      const models = await invoke<SttModelInfo[]>("list_models");
      set({ sttModels: models });
    } catch (error) {
      console.error("Failed to load models:", error);
    } finally {
      set({ isLoadingModels: false });
    }
  },

  downloadModel: async (modelId: string) => {
    set({ downloadingModelId: modelId, downloadProgress: 0 });

    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<DownloadProgress>("model-download-progress", (event) => {
        const p = event.payload;
        if (p.model_id !== modelId) return;

        // Calculate overall progress across all files
        const fileWeight = 1 / p.total_files;
        const fileProgress = p.total_bytes > 0 ? p.downloaded_bytes / p.total_bytes : 0;
        const overall = (p.file_index * fileWeight + fileProgress * fileWeight) * 100;
        set({ downloadProgress: Math.min(Math.round(overall), 100) });
      });

      await invoke("download_model", { modelId });

      set({ downloadProgress: 100 });
      await get().loadModels();
    } catch (error) {
      console.error("Failed to download model:", error);
      throw error;
    } finally {
      if (unlisten) unlisten();
      set({ downloadingModelId: null, downloadProgress: 0 });
    }
  },

  switchModel: async (modelId: string) => {
    try {
      await invoke("switch_model", { modelId });
      await get().loadModels();
      await get().checkModelStatus();
      // Silently reload config without triggering isLoading (which would
      // unmount the page and reset scroll position).
      const config = await invoke<AppConfig>("get_config");
      set({
        apiKey: config.api_key || "",
        apiBaseUrl: config.api_base_url || "https://api.openai.com/v1",
        model: config.model || "gpt-4o-mini",
        language: config.language === "zh" ? "zh-CN" : (config.language || "auto"),
        hotkey: config.hotkey || "Option+Space",
        modelPath: config.model_path || "",
        textStructuring: config.text_structuring ?? true,
        userTags: config.user_tags ?? [],
        customModels: config.custom_models ?? [],
        onboardingCompleted: config.onboarding_completed ?? false,
        inputDevice: config.input_device || "",
        hfEndpoint: config.hf_endpoint || "https://huggingface.co",
        customPromptEnabled: config.custom_prompt_enabled ?? false,
        customPrompt: config.custom_prompt ?? "",
        structuringPrompt: config.structuring_prompt ?? "",
      });
      await get().checkModelRecommendation(get().language);
    } catch (error) {
      console.error("Failed to switch model:", error);
      throw error;
    }
  },

  deleteModel: async (modelId: string) => {
    try {
      await invoke("delete_model", { modelId });
      await get().loadModels();
    } catch (error) {
      console.error("Failed to delete model:", error);
      throw error;
    }
  },

  checkModelRecommendation: async (language: string) => {
    try {
      const rec = await invoke<ModelRecommendation>("get_model_recommendation", { language });
      set({ modelRecommendation: rec.should_switch ? rec : null });
    } catch (error) {
      console.error("Failed to check model recommendation:", error);
      set({ modelRecommendation: null });
    }
  },
}));
