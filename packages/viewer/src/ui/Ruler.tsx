import { type CSSProperties, useEffect, useRef } from "react";
import type { RulerUnit } from "./settings.js";

/**
 * PowerPoint-style ruler. React port of the original `<pptx-ruler>`
 * Lit element. Drawn onto a `<canvas>` so ticks stay sharp at any
 * zoom level. The host (typically `<PptxPresentation>`) drives the
 * geometry inputs:
 *
 * - `orientation` — `"horizontal"` or `"vertical"`.
 * - `slideOriginPx` — distance from the ruler's start edge to the
 *   slide's leading edge, in CSS pixels.
 * - `slideExtentPx` — slide's on-screen extent along the ruler axis.
 * - `slideExtentCm` — slide's intrinsic physical size in cm.
 * - `slideIntrinsicPx` — slide's intrinsic logical pixel size.
 * - `unit` — `"cm"` (centred ±N cm) or `"px"` (origin at slide edge).
 */
export interface RulerProps {
  orientation: "horizontal" | "vertical";
  unit: RulerUnit;
  slideOriginPx: number;
  slideExtentPx: number;
  slideExtentCm: number;
  slideIntrinsicPx: number;
  className?: string;
  style?: CSSProperties;
}

export function Ruler(props: RulerProps): JSX.Element {
  const {
    orientation,
    unit,
    slideOriginPx,
    slideExtentPx,
    slideExtentCm,
    slideIntrinsicPx,
    className,
    style,
  } = props;
  const hostRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const host = hostRef.current;
    const canvas = canvasRef.current;
    if (!host || !canvas) return;
    const draw = (): void => {
      drawRuler(canvas, host, {
        orientation,
        unit,
        slideOriginPx,
        slideExtentPx,
        slideExtentCm,
        slideIntrinsicPx,
      });
    };
    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(host);
    return () => ro.disconnect();
  }, [
    orientation,
    unit,
    slideOriginPx,
    slideExtentPx,
    slideExtentCm,
    slideIntrinsicPx,
  ]);

  return (
    <div
      ref={hostRef}
      className={className}
      style={{ ...rulerHostStyle, ...style }}
    >
      <canvas ref={canvasRef} style={canvasStyle} />
    </div>
  );
}

interface RulerGeom {
  orientation: "horizontal" | "vertical";
  unit: RulerUnit;
  slideOriginPx: number;
  slideExtentPx: number;
  slideExtentCm: number;
  slideIntrinsicPx: number;
}

function drawRuler(
  canvas: HTMLCanvasElement,
  host: HTMLElement,
  geom: RulerGeom,
): void {
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = host.clientWidth;
  const cssHeight = host.clientHeight;
  if (cssWidth <= 0 || cssHeight <= 0) return;
  canvas.width = Math.round(cssWidth * dpr);
  canvas.height = Math.round(cssHeight * dpr);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, cssWidth, cssHeight);

  const isHoriz = geom.orientation === "horizontal";
  const rulerThickness = isHoriz ? cssHeight : cssWidth;

  const slideStart = geom.slideOriginPx;
  const slideExtent = geom.slideExtentPx;
  if (slideExtent <= 0) return;
  const slideEnd = slideStart + slideExtent;

  const fg = getComputedStyle(host).color || "#888";
  const fgFaint = "rgba(127, 127, 127, 0.55)";

  const visibleStart = Math.max(0, slideStart);
  const visibleEnd = Math.min(isHoriz ? cssWidth : cssHeight, slideEnd);
  if (visibleEnd > visibleStart) {
    ctx.fillStyle = "rgba(127, 127, 127, 0.10)";
    if (isHoriz) {
      ctx.fillRect(visibleStart, 0, visibleEnd - visibleStart, rulerThickness);
    } else {
      ctx.fillRect(0, visibleStart, rulerThickness, visibleEnd - visibleStart);
    }
  }

  ctx.strokeStyle = fg;
  ctx.fillStyle = fg;
  ctx.font = "10px system-ui, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.lineWidth = 1;

  if (geom.unit === "cm") {
    drawCm(
      ctx,
      isHoriz,
      rulerThickness,
      slideStart,
      slideEnd,
      geom,
      fg,
      fgFaint,
    );
  } else {
    drawPx(
      ctx,
      isHoriz,
      rulerThickness,
      slideStart,
      slideEnd,
      geom,
      fg,
      fgFaint,
    );
  }
}

function drawCm(
  ctx: CanvasRenderingContext2D,
  isHoriz: boolean,
  rulerThickness: number,
  slideStart: number,
  slideEnd: number,
  geom: RulerGeom,
  fg: string,
  fgFaint: string,
): void {
  const slideExtent = slideEnd - slideStart;
  const slideCm = geom.slideExtentCm;
  if (slideExtent <= 0 || slideCm <= 0) return;
  const pxPerCm = slideExtent / slideCm;
  const slideCenter = slideStart + slideExtent / 2;
  const halfMaxCm = slideCm / 2;
  const showMicro = pxPerCm >= 60;

  const drawTickAt = (cm: number, kind: "major" | "half" | "micro") => {
    if (Math.abs(cm) > halfMaxCm + 1e-3) return;
    const pos = slideCenter + cm * pxPerCm;
    if (pos < slideStart - 1 || pos > slideEnd + 1) return;
    const tickLen =
      kind === "major"
        ? rulerThickness * 0.55
        : kind === "half"
          ? rulerThickness * 0.35
          : rulerThickness * 0.2;
    ctx.strokeStyle = kind === "micro" ? fgFaint : fg;
    ctx.beginPath();
    if (isHoriz) {
      ctx.moveTo(pos + 0.5, rulerThickness);
      ctx.lineTo(pos + 0.5, rulerThickness - tickLen);
    } else {
      ctx.moveTo(rulerThickness, pos + 0.5);
      ctx.lineTo(rulerThickness - tickLen, pos + 0.5);
    }
    ctx.stroke();
    if (kind === "major" && Math.abs(cm) > 0) {
      const label = String(Math.round(Math.abs(cm)));
      ctx.fillStyle = fg;
      if (isHoriz) {
        ctx.fillText(label, pos, rulerThickness * 0.32);
      } else {
        ctx.save();
        ctx.translate(rulerThickness * 0.32, pos);
        ctx.rotate(-Math.PI / 2);
        ctx.fillText(label, 0, 0);
        ctx.restore();
      }
    }
  };

  const totalSteps = Math.ceil(halfMaxCm * 10) + 1;
  for (let dirSign = -1; dirSign <= 1; dirSign += 2) {
    for (let i = 0; i <= totalSteps; i += 1) {
      if (dirSign === -1 && i === 0) continue;
      const cm = i * 0.1 * dirSign;
      const cmAbs = Math.abs(cm);
      const isMajor = Math.abs(cmAbs - Math.round(cmAbs)) < 1e-3;
      const isHalf =
        !isMajor && Math.abs(cmAbs * 2 - Math.round(cmAbs * 2)) < 1e-3;
      const isMicro = !isMajor && !isHalf;
      if (isMicro && !showMicro) continue;
      drawTickAt(cm, isMajor ? "major" : isHalf ? "half" : "micro");
    }
  }
  ctx.fillStyle = fg;
  if (isHoriz) {
    ctx.fillText("0", slideCenter, rulerThickness * 0.32);
  } else {
    ctx.save();
    ctx.translate(rulerThickness * 0.32, slideCenter);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText("0", 0, 0);
    ctx.restore();
  }
}

function drawPx(
  ctx: CanvasRenderingContext2D,
  isHoriz: boolean,
  rulerThickness: number,
  slideStart: number,
  slideEnd: number,
  geom: RulerGeom,
  fg: string,
  fgFaint: string,
): void {
  const visualExtent = slideEnd - slideStart;
  const intrinsicTotal = geom.slideIntrinsicPx;
  if (visualExtent <= 0 || intrinsicTotal <= 0) return;
  const pxPerUnit = visualExtent / intrinsicTotal;
  const desiredSpacingPx = 80;
  const desiredStepUnits = desiredSpacingPx / pxPerUnit;
  const stepUnits = niceStep(desiredStepUnits);
  const minorStepUnits = stepUnits / 5;

  const drawTick = (vUnits: number, isMajor: boolean) => {
    const pos = slideStart + vUnits * pxPerUnit;
    if (pos < slideStart - 1 || pos > slideEnd + 1) return;
    const tickLen = isMajor ? rulerThickness * 0.55 : rulerThickness * 0.25;
    ctx.strokeStyle = isMajor ? fg : fgFaint;
    ctx.beginPath();
    if (isHoriz) {
      ctx.moveTo(pos + 0.5, rulerThickness);
      ctx.lineTo(pos + 0.5, rulerThickness - tickLen);
    } else {
      ctx.moveTo(rulerThickness, pos + 0.5);
      ctx.lineTo(rulerThickness - tickLen, pos + 0.5);
    }
    ctx.stroke();
    if (isMajor) {
      const label = String(Math.round(vUnits));
      ctx.fillStyle = fg;
      if (isHoriz) {
        ctx.fillText(label, pos, rulerThickness * 0.32);
      } else {
        ctx.save();
        ctx.translate(rulerThickness * 0.32, pos);
        ctx.rotate(-Math.PI / 2);
        ctx.fillText(label, 0, 0);
        ctx.restore();
      }
    }
  };

  const totalMinors = Math.round(intrinsicTotal / minorStepUnits);
  for (let i = 0; i <= totalMinors; i += 1) {
    const v = i * minorStepUnits;
    const isMajor = Math.abs(v % stepUnits) < 1e-3 * stepUnits;
    drawTick(v, isMajor);
  }
}

function niceStep(n: number): number {
  if (n <= 0) return 1;
  const exp = Math.floor(Math.log10(n));
  const base = Math.pow(10, exp);
  const norm = n / base;
  let nice: number;
  if (norm < 1.5) nice = 1;
  else if (norm < 3) nice = 2;
  else if (norm < 7) nice = 5;
  else nice = 10;
  return nice * base;
}

const rulerHostStyle: CSSProperties = {
  display: "block",
  width: "100%",
  height: "100%",
  background: "var(--pptx-shell-status-bg, #1f1f23)",
  color: "var(--pptx-shell-status, #888)",
  boxSizing: "border-box",
  overflow: "hidden",
  pointerEvents: "none",
  userSelect: "none",
};

const canvasStyle: CSSProperties = {
  display: "block",
  width: "100%",
  height: "100%",
};
