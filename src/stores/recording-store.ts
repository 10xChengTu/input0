import { create } from "zustand";

type RecordingStatus = "idle" | "recording" | "processing" | "done" | "error";

interface RecordingState {
  status: RecordingStatus;
  pipelineStep: string;
  transcribedText: string;
  optimizedText: string;
  error: string | null;
  audioLevel: number;
  /** Last completed pipeline result — persists across status resets for correction UI */
  lastTranscribedText: string;
  lastOptimizedText: string;
  setStatus: (status: RecordingStatus, step?: string) => void;
  setTranscribedText: (text: string) => void;
  setOptimizedText: (text: string) => void;
  setError: (error: string | null) => void;
  setAudioLevel: (level: number) => void;
  setLastResult: (transcribed: string, optimized: string) => void;
  clearLastResult: () => void;
  reset: () => void;
}

export const useRecordingStore = create<RecordingState>((set) => ({
  status: "idle",
  pipelineStep: "",
  transcribedText: "",
  optimizedText: "",
  error: null,
  audioLevel: 0,
  lastTranscribedText: "",
  lastOptimizedText: "",
  setStatus: (status, step = "") => set({ status, pipelineStep: step }),
  setTranscribedText: (transcribedText) => set({ transcribedText }),
  setOptimizedText: (optimizedText) => set({ optimizedText }),
  setError: (error) => set({ error }),
  setAudioLevel: (audioLevel) => set({ audioLevel }),
  setLastResult: (transcribed, optimized) =>
    set({ lastTranscribedText: transcribed, lastOptimizedText: optimized }),
  clearLastResult: () =>
    set({ lastTranscribedText: "", lastOptimizedText: "" }),
  reset: () =>
    set({
      status: "idle",
      pipelineStep: "",
      transcribedText: "",
      optimizedText: "",
      error: null,
      audioLevel: 0,
    }),
}));
