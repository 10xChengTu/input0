import { useEffect, useRef, useCallback } from "react";

const BAR_HEIGHT = 2;
const CANVAS_HEIGHT = 16;

export function SweepingLight() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animRef = useRef<number>(0);

  const draw = useCallback((time: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.scale(dpr, dpr);

    const t = time / 1000;
    const cycle = 1.8;
    const progress = (t % cycle) / cycle;

    const beamWidth = w * 0.4;
    const centerX = -beamWidth + progress * (w + beamWidth * 2);
    const y = (h - BAR_HEIGHT) / 2;

    const gradient = ctx.createLinearGradient(centerX - beamWidth / 2, 0, centerX + beamWidth / 2, 0);
    gradient.addColorStop(0, "rgba(255,255,255,0)");
    gradient.addColorStop(0.3, "rgba(255,255,255,0.5)");
    gradient.addColorStop(0.5, "rgba(255,255,255,0.8)");
    gradient.addColorStop(0.7, "rgba(255,255,255,0.5)");
    gradient.addColorStop(1, "rgba(255,255,255,0)");

    ctx.fillStyle = "rgba(255,255,255,0.08)";
    ctx.beginPath();
    ctx.roundRect(0, y, w, BAR_HEIGHT, BAR_HEIGHT / 2);
    ctx.fill();

    ctx.save();
    ctx.beginPath();
    ctx.roundRect(0, y, w, BAR_HEIGHT, BAR_HEIGHT / 2);
    ctx.clip();
    ctx.fillStyle = gradient;
    ctx.fillRect(0, y, w, BAR_HEIGHT);
    ctx.restore();

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    animRef.current = requestAnimationFrame(draw);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const observe = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const dpr = window.devicePixelRatio || 1;
      const rect = entry.contentRect;
      canvas.width = rect.width * dpr;
      canvas.height = CANVAS_HEIGHT * dpr;
      canvas.style.height = `${CANVAS_HEIGHT}px`;
    });

    observe.observe(canvas.parentElement || canvas);

    animRef.current = requestAnimationFrame(draw);

    return () => {
      observe.disconnect();
      if (animRef.current) {
        cancelAnimationFrame(animRef.current);
      }
    };
  }, [draw]);

  return <canvas ref={canvasRef} className="block w-full" style={{ height: CANVAS_HEIGHT }} />;
}
