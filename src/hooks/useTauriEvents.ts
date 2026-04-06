import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useRecordingStore } from "../stores/recording-store";
import { useHistoryStore } from "../stores/history-store";

type PipelineState =
  | "idle"
  | "recording"
  | "transcribing"
  | "optimizing"
  | "pasting"
  | "cancelled"
  | { done: { transcribed_text: string; text: string } }
  | { error: { message: string } };

interface PipelineWarning {
  message: string;
}

export function usePipelineEvents() {
  const { setStatus, setOptimizedText, setError, setAudioLevel, setLastResult, reset } = useRecordingStore();

  useEffect(() => {
    const unlisten = listen<{ state: PipelineState }>("pipeline-state", (event) => {
      const { state } = event.payload;

      if (typeof state === "string") {
        switch (state) {
          case "idle":
            reset();
            break;
          case "recording":
            setStatus("recording");
            break;
          case "transcribing":
            setStatus("processing", "Transcribing...");
            break;
          case "optimizing":
            setStatus("processing", "Optimizing...");
            break;
          case "pasting":
            setStatus("processing", "Pasting...");
            break;
          case "cancelled":
            reset();
            break;
        }
      } else if ("done" in state) {
        setOptimizedText(state.done.text);
        setStatus("done");
        if (state.done.text) {
          setLastResult(state.done.transcribed_text, state.done.text);
          useHistoryStore.getState().saveResult(
            state.done.transcribed_text,
            state.done.text
          );
        }
      } else if ("error" in state) {
        setError(state.error.message);
        setStatus("error");
      }
    });

    const unlistenLevel = listen<{ level: number }>("audio-level", (event) => {
      setAudioLevel(event.payload.level);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenLevel.then((fn) => fn());
    };
  }, [setStatus, setOptimizedText, setError, setAudioLevel, setLastResult, reset]);
}

export function usePipelineWarnings(onWarning: (message: string) => void) {
  useEffect(() => {
    const unlisten = listen<PipelineWarning>("pipeline-warning", (event) => {
      onWarning(event.payload.message);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onWarning]);
}

