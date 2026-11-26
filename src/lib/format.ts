import type { Status } from "./types";
import type { Tone } from "../theme";

const oneDecimal = new Intl.NumberFormat("es", { maximumFractionDigits: 1 });
const noDecimals = new Intl.NumberFormat("es", { maximumFractionDigits: 0 });

export function bytes(n: number | null | undefined): string {
  if (n == null) return "—";
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = n / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${(value >= 100 ? noDecimals : oneDecimal).format(value)} ${units[unit]}`;
}

export function duration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "";
  if (seconds < 60) return `${Math.ceil(seconds)} s`;
  if (seconds < 3600) return `${Math.ceil(seconds / 60)} min`;
  return `${Math.floor(seconds / 3600)} h ${Math.round((seconds % 3600) / 60)} min`;
}

export function ago(timestamp: number | null, now: number): string {
  if (!timestamp) return "nunca";
  const seconds = Math.max(0, Math.round((now - timestamp) / 1000));
  if (seconds < 45) return "hace un momento";
  if (seconds < 3600) return `hace ${Math.max(1, Math.round(seconds / 60))} min`;
  if (seconds < 86400) return `hace ${Math.round(seconds / 3600)} h`;
  return `hace ${Math.round(seconds / 86400)} d`;
}

/** `m:ss` until `target`. */
export function countdown(target: number | null, now: number): string {
  if (!target) return "—:——";
  const seconds = Math.max(0, Math.round((target - now) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export function longDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString("es", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

/** Something readable to show until Rapidgator tells us the real file name. */
export function nameFromUrl(url: string): string {
  try {
    const segments = new URL(url).pathname.split("/").filter(Boolean);
    if (segments.length >= 3) return decodeURIComponent(segments[2]).replace(/\.html$/i, "");
  } catch {
    // Not a parseable URL: fall through and show it as it is.
  }
  return url.replace(/^https?:\/\//, "");
}

export const STATUS: Record<Status, { label: string; tone: Tone }> = {
  pending: { label: "Sin comprobar", tone: "muted" },
  available: { label: "Lista para descargar", tone: "ok" },
  unavailable: { label: "Aún no se puede descargar", tone: "warn" },
  missing: { label: "No existe", tone: "bad" },
  downloading: { label: "Descargando", tone: "info" },
  completed: { label: "Descargado", tone: "muted" },
  failed: { label: "La descarga falló", tone: "bad" },
};

export const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
