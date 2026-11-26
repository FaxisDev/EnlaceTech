import { useEffect, useState } from "react";
import { call, on } from "../lib/bridge";
import type { Progress, Snapshot } from "../lib/types";

/** The backend's state, kept up to date by the events it emits. */
export function useSnapshot(): Snapshot | null {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);

  useEffect(() => {
    let alive = true;
    const unlisten: Array<() => void> = [];
    const keep = (off: () => void) => (alive ? unlisten.push(off) : off());

    on<Snapshot>("snapshot", (next) => alive && setSnapshot(next)).then(keep);

    // Download progress arrives several times a second; it only touches one link.
    on<Progress>("progress", (p) => {
      if (!alive) return;
      setSnapshot(
        (current) =>
          current && {
            ...current,
            links: current.links.map((l) =>
              l.id === p.id ? { ...l, received: p.received, speed: p.speed, size: l.size ?? p.total } : l,
            ),
          },
      );
    }).then(keep);

    call<Snapshot>("get_snapshot")
      // An event may already have delivered something newer.
      .then((first) => alive && setSnapshot((current) => current ?? first))
      .catch(() => {});

    return () => {
      alive = false;
      unlisten.forEach((off) => off());
    };
  }, []);

  return snapshot;
}

/** The current time, refreshed every `everyMs`. */
export function useNow(everyMs: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), everyMs);
    return () => window.clearInterval(timer);
  }, [everyMs]);
  return now;
}
