import Alert from "@mui/material/Alert";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import ButtonBase from "@mui/material/ButtonBase";
import InputBase from "@mui/material/InputBase";
import Typography from "@mui/material/Typography";
import AddRounded from "@mui/icons-material/AddRounded";
import { useMemo, useState } from "react";
import type { KeyboardEvent } from "react";

import { useNow } from "../hooks/useSnapshot";
import { plural } from "../lib/format";
import type { AddReport, LinkItem, Run, Say, Snapshot, Status } from "../lib/types";
import { navy, tones } from "../theme";
import { Mark } from "./Brand";
import LinkRow from "./LinkRow";

type Filter = "all" | "ready" | "waiting" | "done";

const GROUP: Record<Status, Exclude<Filter, "all">> = {
  available: "ready",
  downloading: "ready",
  pending: "waiting",
  unavailable: "waiting",
  missing: "waiting",
  failed: "waiting",
  completed: "done",
};

const FILTERS: Array<{ key: Filter; label: string }> = [
  { key: "all", label: "Todos" },
  { key: "ready", label: "Disponibles" },
  { key: "waiting", label: "En espera" },
  { key: "done", label: "Descargados" },
];

interface Props {
  snapshot: Snapshot;
  run: Run;
  say: Say;
  onSignIn: () => void;
}

export default function LinksView({ snapshot, run, say, onSignIn }: Props) {
  const now = useNow(20_000);
  const [filter, setFilter] = useState<Filter>("all");
  const { links } = snapshot;

  const counts = useMemo(() => {
    const totals: Record<Filter, number> = { all: links.length, ready: 0, waiting: 0, done: 0 };
    for (const l of links) totals[GROUP[l.status]] += 1;
    return totals;
  }, [links]);

  const shown = filter === "all" ? links : links.filter((l) => GROUP[l.status] === filter);

  return (
    <>
      <Box sx={{ px: 4, pb: 2.5 }}>
        <Typography variant="h5" component="h1">
          Enlaces
        </Typography>
        <Typography sx={{ color: "text.secondary", mt: 0.25 }}>{summary(links)}</Typography>

        <AddBar run={run} say={say} />

        {snapshot.ready && !snapshot.signedIn && (
          <Alert
            severity="info"
            variant="outlined"
            sx={{ mt: 1.5, borderColor: navy.line, color: "text.primary" }}
            action={
              <Button size="small" onClick={onSignIn}>
                Iniciar sesión
              </Button>
            }
          >
            Inicia sesión en Rapidgator para saber cuáles de tus enlaces se pueden descargar.
          </Alert>
        )}
        {snapshot.notice && (
          <Alert severity="warning" variant="outlined" sx={{ mt: 1.5, borderColor: "rgba(242, 182, 92, 0.35)", color: "text.primary" }}>
            {snapshot.notice}
          </Alert>
        )}
      </Box>

      {links.length > 0 && (
        <Box sx={{ px: 4, display: "flex", alignItems: "center", gap: 0.5, pb: 0.5 }}>
          {FILTERS.map(({ key, label }) => (
            <FilterPill
              key={key}
              label={label}
              count={counts[key]}
              active={filter === key}
              onClick={() => setFilter(key)}
            />
          ))}
          <Box sx={{ flex: 1 }} />
          {counts.done > 0 && (
            <Button size="small" sx={{ color: "text.secondary" }} onClick={() => run("clear_completed")}>
              Quitar descargados
            </Button>
          )}
        </Box>
      )}

      <Box sx={{ flex: 1, minHeight: 0, overflowY: "auto", px: 3, pb: 3 }}>
        {links.length === 0 ? (
          <Empty
            title="Aún no hay enlaces"
            text="Pega arriba los enlaces de Rapidgator que quieras guardar. EnlaceTech irá comprobando cuándo se pueden descargar."
          />
        ) : shown.length === 0 ? (
          <Empty title="Nada por aquí" text="No hay enlaces en este grupo ahora mismo." />
        ) : (
          <Box component="ul" sx={{ listStyle: "none", m: 0, p: 0 }}>
            {shown.map((link) => (
              <LinkRow key={link.id} link={link} now={now} run={run} />
            ))}
          </Box>
        )}
      </Box>
    </>
  );
}

function summary(links: LinkItem[]): string {
  if (links.length === 0) return "Guarda enlaces y descárgalos en cuanto se pueda.";
  const ready = links.filter((l) => l.status === "available").length;
  const active = links.filter((l) => l.status === "downloading").length;
  const waiting = links.filter((l) => GROUP[l.status] === "waiting").length;
  const parts = [
    ready > 0 ? plural(ready, "lista para descargar", "listas para descargar") : null,
    active > 0 ? plural(active, "descargándose", "descargándose") : null,
    waiting > 0 ? `${waiting} en espera` : null,
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : "Todo descargado.";
}

function AddBar({ run, say }: { run: Run; say: Say }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit() {
    const value = text.trim();
    if (!value || busy) return;
    setBusy(true);
    const report = await run<AddReport>("add_links", { text: value });
    setBusy(false);
    if (!report) return;

    const said: string[] = [];
    if (report.added > 0) said.push(plural(report.added, "enlace añadido", "enlaces añadidos"));
    if (report.duplicates > 0) said.push(plural(report.duplicates, "ya estaba en la lista", "ya estaban en la lista"));
    if (report.invalid.length > 0) {
      said.push(plural(report.invalid.length, "no es un enlace de Rapidgator", "no son enlaces de Rapidgator"));
    }
    say(said.join(" · "), report.invalid.length > 0 ? "error" : report.added > 0 ? "success" : "info");

    // Keep what could not be added so it can be corrected.
    setText(report.invalid.join("\n"));
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void submit();
    }
  }

  return (
    <Box
      sx={{
        mt: 2.5,
        display: "flex",
        alignItems: "flex-end",
        gap: 1,
        bgcolor: navy.surface,
        border: `1px solid ${navy.line}`,
        borderRadius: "12px",
        py: 0.75,
        pl: 2,
        pr: 0.75,
        transition: "border-color 120ms",
        "&:focus-within": { borderColor: tones.info.fg },
      }}
    >
      <InputBase
        multiline
        maxRows={4}
        fullWidth
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder="Pega uno o varios enlaces de Rapidgator…"
        spellCheck={false}
        sx={{ fontSize: 13.5, py: 0.75, "& textarea::placeholder": { color: "text.disabled", opacity: 1 } }}
        slotProps={{ input: { "aria-label": "Enlaces de Rapidgator para añadir" } }}
      />
      <Button
        variant="contained"
        startIcon={<AddRounded />}
        disabled={!text.trim() || busy}
        onClick={() => void submit()}
      >
        Añadir
      </Button>
    </Box>
  );
}

function FilterPill(props: { label: string; count: number; active: boolean; onClick: () => void }) {
  const { label, count, active, onClick } = props;
  return (
    <ButtonBase
      onClick={onClick}
      aria-pressed={active}
      sx={{
        height: 30,
        px: 1.4,
        gap: 0.9,
        borderRadius: "15px",
        fontSize: 12.5,
        fontWeight: active ? 640 : 520,
        color: active ? "text.primary" : "text.secondary",
        bgcolor: active ? navy.raised : "transparent",
        transition: "background-color 120ms, color 120ms",
        "&:hover": { color: "text.primary", bgcolor: active ? navy.raised : "rgba(120, 160, 230, 0.07)" },
        "&.Mui-focusVisible": { outline: `2px solid ${tones.info.fg}`, outlineOffset: 1 },
      }}
    >
      {label}
      <Box component="span" sx={{ color: active ? "text.secondary" : "text.disabled", fontWeight: 600 }}>
        {count}
      </Box>
    </ButtonBase>
  );
}

function Empty({ title, text }: { title: string; text: string }) {
  return (
    <Box sx={{ height: "100%", minHeight: 240, display: "grid", placeItems: "center", textAlign: "center" }}>
      <Box sx={{ maxWidth: 360, display: "grid", justifyItems: "center", gap: 1 }}>
        <Mark size={52} faint />
        <Typography variant="h6" sx={{ mt: 1 }}>
          {title}
        </Typography>
        <Typography sx={{ color: "text.secondary", textWrap: "balance" }}>{text}</Typography>
      </Box>
    </Box>
  );
}
