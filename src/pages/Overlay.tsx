import { useEffect, useState, useMemo, useCallback } from "react";
import { useRecordingStore } from "../stores/recording-store";
import { usePipelineEvents, usePipelineWarnings } from "../hooks/useTauriEvents";
import { WaveformAnimation } from "../components/WaveformAnimation";
import { SweepingLight } from "../components/ProcessingIndicator";
import { motion, AnimatePresence } from "framer-motion";
import { useLocaleStore } from "../i18n";

type VisualPhase = "recording" | "processing" | "error" | "warning";

const pillStyle = {
  backgroundColor: "rgba(0,0,0,1)",
  borderColor: "rgba(255,255,255,0.20)",
  boxShadow: "0 8px 32px rgba(0,0,0,0.4), inset 0 1px 0 rgba(255,255,255,0.08)",
};

function classifyError(raw: string, t: ReturnType<typeof useLocaleStore.getState>["t"]): string {
  const lower = raw.toLowerCase();

  if (lower.includes("not loaded") || lower.includes("not initialized") || lower.includes("no stt model")) {
    return t.overlay.modelNotLoaded;
  }
  if (lower.includes("failed to load whisper") || lower.includes("failed to load") && lower.includes("model")) {
    return t.overlay.modelLoadFailed;
  }
  if (lower.includes("transcription failed") || lower.includes("failed to get segment")) {
    return t.overlay.transcriptionFailed;
  }
  if (lower.includes("no default input device")) {
    return t.overlay.noMicrophone;
  }
  if (lower.includes("audio error") || lower.includes("failed to build") && lower.includes("stream")) {
    return t.overlay.audioError;
  }
  if (lower.includes("network error")) {
    return t.overlay.networkError;
  }
  if (lower.includes("llm error") || lower.includes("failed to parse response") || lower.includes("response missing")) {
    return t.overlay.llmError;
  }
  if (lower.includes("clipboard") || lower.includes("applescript") || lower.includes("input error")) {
    return t.overlay.pasteError;
  }
  if (lower.includes("configuration error") || lower.includes("config")) {
    return t.overlay.configError;
  }
  if (lower.includes("cancelled")) {
    return t.overlay.cancelled;
  }

  return t.overlay.genericError;
}

function Overlay() {
  const { status, error, audioLevel } = useRecordingStore();
  const { t } = useLocaleStore();
  const [showError, setShowError] = useState(false);
  const [warningMessage, setWarningMessage] = useState<string | null>(null);
  usePipelineEvents();

  const onWarning = useCallback(() => {
    setWarningMessage(t.overlay.optimizationSkipped);
  }, [t]);

  usePipelineWarnings(onWarning);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  }, []);

  useEffect(() => {
    if (status === "error") {
      if (error) {
        console.error("[Input0] Pipeline error:", error);
      }
      setShowError(true);
    } else {
      setShowError(false);
    }
  }, [status]);

  // Auto-dismiss warning after 3 seconds
  useEffect(() => {
    if (!warningMessage) return;
    const timer = setTimeout(() => setWarningMessage(null), 3000);
    return () => clearTimeout(timer);
  }, [warningMessage]);

  const isBarVisible = status === "recording" || status === "processing" || showError || !!warningMessage;

  const phase: VisualPhase = useMemo(() => {
    if (showError) return "error";
    if (warningMessage) return "warning";
    if (status === "recording") return "recording";
    return "processing";
  }, [status, showError, warningMessage]);

  const errorMessage = useMemo(() => {
    if (!error) return null;
    return classifyError(error, t);
  }, [error, t]);

  if (!isBarVisible) return null;

  return (
    <div className="flex items-end justify-center h-screen pb-3 w-full select-none pointer-events-none">
      <div className="flex flex-col items-center gap-1">
        <AnimatePresence>
          {isBarVisible && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9, y: 12 }}
              animate={{
                opacity: 1,
                scale: 1,
                y: 0,
                backgroundColor: pillStyle.backgroundColor,
                borderColor: pillStyle.borderColor,
                boxShadow: pillStyle.boxShadow,
              }}
              exit={{ opacity: 0, scale: 0.9, y: 12 }}
              transition={{
                opacity: { duration: 0.2 },
                scale: { type: "spring", stiffness: 400, damping: 30, mass: 0.5 },
                y: { type: "spring", stiffness: 400, damping: 30, mass: 0.5 },
                backgroundColor: { duration: 0.3, ease: "easeInOut" },
                borderColor: { duration: 0.3, ease: "easeInOut" },
                boxShadow: { duration: 0.3, ease: "easeInOut" },
              }}
              className={`relative flex items-center justify-center ${phase === "error" || phase === "warning" ? "min-w-[120px] w-auto px-4" : "w-[120px]"} h-[32px] rounded-full text-white border overflow-hidden`}
            >
              <AnimatePresence initial={false}>
                {phase === "recording" && (
                  <motion.div
                    key="recording"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.15 }}
                    className="absolute inset-0 flex items-center pl-[16px]"
                  >
                    <div className="relative flex-shrink-0 flex h-2 w-2">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-2 w-2 bg-red-500"></span>
                    </div>
                    <div className="flex-1 flex justify-center pr-[16px]">
                      <WaveformAnimation level={audioLevel} />
                    </div>
                  </motion.div>
                )}
                {phase === "processing" && (
                  <motion.div
                    key="thinking"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.15 }}
                    className="absolute inset-0 flex items-center px-[16px]"
                  >
                    <SweepingLight />
                  </motion.div>
                )}
                {phase === "error" && (
                  <motion.div
                    key="error-label"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.15 }}
                    className="absolute inset-0 flex items-center justify-center px-4"
                  >
                    <span className="text-white/80 text-xs font-semibold tracking-wide whitespace-nowrap">{errorMessage || "Error"}</span>
                  </motion.div>
                )}
                {phase === "warning" && (
                  <motion.div
                    key="warning-label"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.15 }}
                    className="absolute inset-0 flex items-center justify-center px-4"
                  >
                    <span className="text-amber-400/90 text-xs font-semibold tracking-wide whitespace-nowrap">{warningMessage}</span>
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default Overlay;
