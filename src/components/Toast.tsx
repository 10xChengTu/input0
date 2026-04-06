import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

export interface ToastProps {
  message: string;
  type: "success" | "error";
  onDismiss: () => void;
}

const quickSpring = { stiffness: 400, damping: 30, mass: 0.5 };

export function Toast({ message, type, onDismiss }: ToastProps) {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const showTimer = setTimeout(() => setIsVisible(true), 10);
    const hideTimer = setTimeout(() => setIsVisible(false), 2700);
    const removeTimer = setTimeout(() => onDismiss(), 3000);

    return () => {
      clearTimeout(showTimer);
      clearTimeout(hideTimer);
      clearTimeout(removeTimer);
    };
  }, [onDismiss]);

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          initial={{ opacity: 0, x: 40 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: 40 }}
          transition={{ type: "spring", ...quickSpring }}
          className={`fixed top-6 right-6 z-50 flex items-center gap-3 pl-3 pr-6 py-3 rounded-lg border ${
            type === "error"
              ? "bg-[var(--theme-toast-bg)] border-[var(--theme-toast-error-border)]"
              : "bg-[var(--theme-toast-bg)] border-[var(--theme-toast-border)]"
          }`}
        >
          <div className="flex items-center justify-center w-5 h-5 shrink-0">
            {type === "success" ? (
              <svg className="w-4 h-4 text-[var(--theme-toast-icon)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            ) : (
              <svg className="w-4 h-4 text-[var(--theme-toast-error-text)]" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
              </svg>
            )}
          </div>
          <p className={`text-sm font-medium tracking-tight ${
            type === "error" ? "text-[var(--theme-toast-error-text)]" : "text-[var(--theme-toast-text)]"
          }`}>{message}</p>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
