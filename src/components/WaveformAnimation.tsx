import { useEffect, useRef, useCallback } from "react";

const BAR_COUNT = 20;
const BAR_WIDTH = 1.5;
const BAR_GAP = 1;
const MIN_HEIGHT = 2;
const MAX_HEIGHT = 12;
const CANVAS_HEIGHT = 16;
const SMOOTHING = 0.25;

interface WaveformAnimationProps {
  level?: number;
}

export function WaveformAnimation({ level = 0 }: WaveformAnimationProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);
  const phasesRef = useRef<number[]>([]);
  const smoothedLevelRef = useRef<number>(0);
  const levelRef = useRef<number>(0);

  levelRef.current = level;

  if (phasesRef.current.length === 0) {
    phasesRef.current = Array.from({ length: BAR_COUNT }, () => Math.random() * Math.PI * 2);
  }

  const draw = useCallback((time: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const h = canvas.height / dpr;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.scale(dpr, dpr);

    const t = time / 1000;
    const target = levelRef.current;
    smoothedLevelRef.current += (target - smoothedLevelRef.current) * SMOOTHING;
    const sLevel = smoothedLevelRef.current;

    for (let i = 0; i < BAR_COUNT; i++) {
      const phase = phasesRef.current[i];
      const pos = i / (BAR_COUNT - 1);

      // Multi-frequency sine superposition: each frequency creates a different
      // timescale of movement (slow sway, medium ripple, fast shimmer, breath).
      // The specific freq/weight combos (2.2×0.35, 3.7×0.25, 5.9×0.15, 0.8×0.15)
      // are tuned to avoid visible repetition cycles within ~30s of viewing.
      const f1 = Math.sin(t * 2.2 + phase + pos * Math.PI * 1.5) * 0.35;
      const f2 = Math.sin(t * 3.7 + phase * 1.3 + pos * Math.PI * 2.8) * 0.25;
      const f3 = Math.sin(t * 5.9 + phase * 2.1 + pos * Math.PI * 4.2) * 0.15;
      const f4 = Math.sin(t * 0.8 + phase * 0.5) * 0.15;
      const pulse = Math.pow(Math.sin(t * 1.5 + phase * 3.0 + pos * 6), 8) * 0.1;

      const combined = 0.5 + f1 + f2 + f3 + f4 + pulse;
      const texture = Math.max(0, Math.min(1, combined));

      const centerBias = 1 - Math.pow((pos - 0.5) * 2, 2) * 0.35;

      const amplitude = 0.08 + 0.92 * sLevel;
      const barH = MIN_HEIGHT + (MAX_HEIGHT - MIN_HEIGHT) * texture * centerBias * amplitude;
      const barX = i * (BAR_WIDTH + BAR_GAP);
      const barY = (h - barH) / 2;

      const alpha = 0.5 + 0.5 * (barH / MAX_HEIGHT);
      ctx.fillStyle = `rgba(255, 255, 255, ${alpha})`;
      ctx.beginPath();
      ctx.roundRect(barX, barY, BAR_WIDTH, barH, BAR_WIDTH / 2);
      ctx.fill();
    }

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    animRef.current = requestAnimationFrame(draw);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const totalWidth = BAR_COUNT * (BAR_WIDTH + BAR_GAP) - BAR_GAP;

    canvas.width = totalWidth * dpr;
    canvas.height = CANVAS_HEIGHT * dpr;
    canvas.style.width = `${totalWidth}px`;
    canvas.style.height = `${CANVAS_HEIGHT}px`;

    animRef.current = requestAnimationFrame(draw);

    return () => {
      if (animRef.current) {
        cancelAnimationFrame(animRef.current);
      }
    };
  }, [draw]);

  return <canvas ref={canvasRef} className="block" />;
}
