// Development-only stand-in for the Rust backend, used when the page runs in a
// plain browser (`npm run dev` without Tauri). It is loaded behind
// `import.meta.env.DEV`, so production builds do not include it.
//
// Query string switches: `?signedout` and `?empty`.

import type { AddReport, LinkItem, Snapshot } from "./types";

type Handler = (payload: unknown) => void;

const query = new URLSearchParams(window.location.search);
const now = Date.now();
const MB = 1024 * 1024;
const GB = 1024 * MB;

function link(partial: Partial<LinkItem> & { id: string }): LinkItem {
  return {
    url: `https://rapidgator.net/file/${partial.id}/archivo.html`,
    fileId: partial.id,
    name: null,
    size: null,
    status: "pending",
    reason: null,
    rawReason: null,
    addedAt: now,
    checkedAt: now - 3 * 60_000,
    savedPath: null,
    checking: false,
    received: 0,
    speed: 0,
    ...partial,
  };
}

const sample: LinkItem[] = [
  link({
    id: "a1",
    name: "Documental.Oceanos.2160p.part1.rar",
    size: 4.2 * GB,
    status: "available",
  }),
  link({
    id: "a2",
    name: "Documental.Oceanos.2160p.part2.rar",
    size: 4.2 * GB,
    status: "downloading",
    received: 1.6 * GB,
    speed: 11.4 * MB,
  }),
  link({
    id: "a3",
    name: "Curso.Fotografia.Nocturna.zip",
    size: 812 * MB,
    status: "unavailable",
    reason: "Se agotó el tráfico de descarga de tu cuenta. Se volverá a intentar en la próxima comprobación.",
    rawReason: "You have reached quota of premium downloaded information",
  }),
  link({
    id: "a4",
    name: "Plantillas.Presentaciones.2026.7z",
    size: 96 * MB,
    status: "unavailable",
    reason: "Rapidgator solo permite esta descarga con una cuenta premium.",
    rawReason: "This file can be downloaded by premium only",
  }),
  link({
    id: "a5",
    url: "https://rapidgator.net/file/a5/Sesion.Estudio.Master.flac.html",
    status: "missing",
    reason: "El archivo no existe en Rapidgator: fue eliminado, es privado o el enlace está mal escrito.",
    rawReason: "NO ACCESS",
  }),
  link({
    id: "a6",
    name: "Manual.Usuario.v3.pdf",
    size: 18.5 * MB,
    status: "completed",
    received: 18.5 * MB,
    savedPath: "/Users/tu/Downloads/Manual.Usuario.v3.pdf",
  }),
  link({
    id: "a7",
    name: "Backup.Proyecto.tar.gz",
    size: 2.3 * GB,
    status: "failed",
    reason: "La conexión se cortó antes de terminar la descarga.",
  }),
  link({
    id: "a8",
    url: "https://rapidgator.net/file/a8/Recien.Anadido.mkv.html",
    status: "pending",
    checkedAt: null,
    checking: true,
  }),
];

const signedOut = query.has("signedout");

let snap: Snapshot = {
  links: query.has("empty") ? [] : sample,
  downloadDir: "/Users/tu/Downloads",
  defaultDir: true,
  intervalMinutes: 15,
  rememberPassword: true,
  notifications: true,
  keepInTray: true,
  account: signedOut
    ? null
    : { email: "tu@correo.com", isPremium: true, premiumEnd: 1830000000, trafficLeft: 38 * GB, trafficTotal: 120 * GB },
  signedIn: !signedOut,
  ready: true,
  checking: false,
  lastCheck: now - 3 * 60_000,
  nextCheck: now + 12 * 60_000,
  notice: null,
};

const listeners = new Map<string, Set<Handler>>();

function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((handler) => handler(payload));
}

function update(change: (s: Snapshot) => Snapshot) {
  snap = change(snap);
  emit("snapshot", snap);
}

function patch(id: string, change: Partial<LinkItem>) {
  update((s) => ({ ...s, links: s.links.map((l) => (l.id === id ? { ...l, ...change } : l)) }));
}

// Keep the sample download moving.
window.setInterval(() => {
  for (const l of snap.links) {
    if (l.status !== "downloading" || !l.size) continue;
    const received = Math.min(l.size, l.received + l.speed * 0.5);
    if (received >= l.size) {
      patch(l.id, { status: "completed", received, speed: 0, savedPath: `${snap.downloadDir}/${l.name}` });
    } else {
      snap = { ...snap, links: snap.links.map((x) => (x.id === l.id ? { ...x, received } : x)) };
      emit("progress", { id: l.id, received, total: l.size, speed: l.speed });
    }
  }
}, 500);

export async function call(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const id = args.id as string | undefined;
  switch (cmd) {
    case "get_snapshot":
      return snap;
    case "add_links": {
      const report: AddReport = { added: 0, duplicates: 0, invalid: [] };
      const fresh: LinkItem[] = [];
      for (const raw of String(args.text ?? "").split(/\s+/).filter(Boolean)) {
        const match = /rapidgator\.net\/file\/([a-z0-9]+)/i.exec(raw) ?? /rg\.to\/file\/([a-z0-9]+)/i.exec(raw);
        if (!match) report.invalid.push(raw);
        else if ([...snap.links, ...fresh].some((l) => l.fileId === match[1])) report.duplicates += 1;
        else {
          report.added += 1;
          fresh.push(link({ id: match[1], url: raw, checkedAt: null, checking: true }));
        }
      }
      update((s) => ({ ...s, links: [...fresh, ...s.links] }));
      window.setTimeout(() => fresh.forEach((l) => patch(l.id, { checking: false, status: "available", size: 640 * MB, checkedAt: Date.now() })), 1400);
      return report;
    }
    case "remove_link":
      update((s) => ({ ...s, links: s.links.filter((l) => l.id !== id) }));
      return;
    case "clear_completed":
      update((s) => ({ ...s, links: s.links.filter((l) => l.status !== "completed") }));
      return;
    case "check_now":
      if (!snap.signedIn) throw "Inicia sesión en Ajustes para comprobar los enlaces.";
      if (id) {
        patch(id, { checking: true });
        window.setTimeout(() => patch(id, { checking: false, checkedAt: Date.now() }), 1200);
      } else {
        update((s) => ({ ...s, checking: true }));
        window.setTimeout(
          () => update((s) => ({ ...s, checking: false, lastCheck: Date.now(), nextCheck: Date.now() + s.intervalMinutes * 60_000 })),
          1600,
        );
      }
      return;
    case "start_download":
      if (id) patch(id, { status: "downloading", received: 0, speed: 9.6 * MB, reason: null, rawReason: null });
      return;
    case "cancel_download":
      if (id) patch(id, { status: "available", received: 0, speed: 0 });
      return;
    case "login":
      if (!args.password) throw "Escribe tu correo y tu contraseña de Rapidgator.";
      update((s) => ({
        ...s,
        signedIn: true,
        account: { email: String(args.email), isPremium: false, premiumEnd: null, trafficLeft: null, trafficTotal: null },
      }));
      return;
    case "logout":
      update((s) => ({ ...s, signedIn: false, account: null }));
      return;
    case "pick_download_dir":
      update((s) => ({ ...s, downloadDir: "/Volumes/Externo/Descargas", defaultDir: false }));
      return;
    case "reset_download_dir":
      update((s) => ({ ...s, downloadDir: "/Users/tu/Downloads", defaultDir: true }));
      return;
    case "set_interval":
      update((s) => ({ ...s, intervalMinutes: Number(args.minutes), nextCheck: Date.now() + Number(args.minutes) * 60_000 }));
      return;
    case "set_keep_in_tray":
      update((s) => ({ ...s, keepInTray: Boolean(args.enabled) }));
      return;
    case "set_notifications":
      update((s) => ({ ...s, notifications: Boolean(args.enabled) }));
      return;
    default:
      return;
  }
}

export async function on(event: string, handler: Handler): Promise<() => void> {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(handler);
  return () => listeners.get(event)?.delete(handler);
}
