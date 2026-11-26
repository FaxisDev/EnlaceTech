import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** True inside the desktop app; false when the page is opened in a plain browser. */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const isMac = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);

type Unlisten = () => void;

/**
 * Calls a Rust command. In a plain browser during development it talks to a
 * small in-memory stand-in instead, so the interface can be worked on with
 * `npm run dev` alone. The stand-in is not part of production builds.
 */
export async function call<T = void>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  if (import.meta.env.DEV) {
    const mock = await import("./mock");
    return mock.call(cmd, args) as Promise<T>;
  }
  throw new Error("EnlaceTech solo funciona dentro de la app de escritorio.");
}

export async function on<T>(event: string, handler: (payload: T) => void): Promise<Unlisten> {
  if (inTauri) return listen<T>(event, (e) => handler(e.payload));
  if (import.meta.env.DEV) {
    const mock = await import("./mock");
    return mock.on(event, handler as (payload: unknown) => void);
  }
  return () => {};
}

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Algo salió mal.";
}
