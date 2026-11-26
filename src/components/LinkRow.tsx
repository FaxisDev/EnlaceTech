import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import IconButton from "@mui/material/IconButton";
import LinearProgress from "@mui/material/LinearProgress";
import Tooltip from "@mui/material/Tooltip";
import Typography from "@mui/material/Typography";
import CheckRounded from "@mui/icons-material/CheckRounded";
import CloseRounded from "@mui/icons-material/CloseRounded";
import DeleteOutlineRounded from "@mui/icons-material/DeleteOutlineRounded";
import DownloadRounded from "@mui/icons-material/DownloadRounded";
import ErrorOutlineRounded from "@mui/icons-material/ErrorOutlineRounded";
import FolderOpenRounded from "@mui/icons-material/FolderOpenRounded";
import LinkOffRounded from "@mui/icons-material/LinkOffRounded";
import MoreHorizRounded from "@mui/icons-material/MoreHorizRounded";
import RefreshRounded from "@mui/icons-material/RefreshRounded";
import ScheduleRounded from "@mui/icons-material/ScheduleRounded";
import SouthRounded from "@mui/icons-material/SouthRounded";
import type { ReactElement } from "react";

import { ago, bytes, duration, nameFromUrl, STATUS } from "../lib/format";
import type { LinkItem, Run, Status } from "../lib/types";
import { mono, navy, spin, tones } from "../theme";

const GLYPH: Record<Status, ReactElement> = {
  pending: <MoreHorizRounded />,
  available: <DownloadRounded />,
  unavailable: <ScheduleRounded />,
  missing: <LinkOffRounded />,
  downloading: <SouthRounded />,
  completed: <CheckRounded />,
  failed: <ErrorOutlineRounded />,
};

interface Props {
  link: LinkItem;
  now: number;
  run: Run;
}

export default function LinkRow({ link, now, run }: Props) {
  const { label, tone } = STATUS[link.status];
  const color = tones[tone];
  const title = link.name ?? nameFromUrl(link.url);
  const downloading = link.status === "downloading";
  const canRecheck = !downloading && link.status !== "completed";

  return (
    <Box
      component="li"
      sx={{
        display: "grid",
        gridTemplateColumns: "40px minmax(0, 1fr) auto",
        columnGap: 1.75,
        alignItems: "start",
        py: 1.75,
        px: 1,
        borderBottom: `1px solid ${navy.lineSoft}`,
        "&:last-of-type": { borderBottom: "none" },
      }}
    >
      <Box
        sx={{
          width: 40,
          height: 40,
          borderRadius: "12px",
          bgcolor: color.wash,
          color: color.fg,
          display: "grid",
          placeItems: "center",
          "& svg": { fontSize: 20 },
        }}
      >
        {GLYPH[link.status]}
      </Box>

      <Box sx={{ minWidth: 0, pt: "1px" }}>
        <Typography variant="subtitle1" noWrap title={title}>
          {title}
        </Typography>

        <Typography variant="body2" noWrap sx={{ color: "text.secondary" }}>
          <Box component="span" sx={{ color: color.fg, fontWeight: 600 }}>
            {link.checking ? "Comprobando…" : label}
          </Box>
          {link.size != null && !downloading && <> · {bytes(link.size)}</>}
          {!downloading && !link.checking && link.status !== "completed" && link.checkedAt && (
            <> · comprobado {ago(link.checkedAt, now)}</>
          )}
        </Typography>

        {downloading && <Transfer link={link} />}

        {link.reason && (
          <Typography
            variant="body2"
            sx={{
              mt: 0.75,
              color: link.status === "available" || downloading ? "text.secondary" : color.fg,
              userSelect: "text",
              WebkitUserSelect: "text",
              cursor: "text",
              textWrap: "pretty",
            }}
          >
            {link.reason}
          </Typography>
        )}
        {link.rawReason && (
          <Typography
            variant="caption"
            sx={{
              display: "block",
              mt: 0.25,
              color: "text.disabled",
              fontFamily: mono,
              userSelect: "text",
              WebkitUserSelect: "text",
              cursor: "text",
              overflowWrap: "anywhere",
            }}
          >
            Rapidgator: «{link.rawReason}»
          </Typography>
        )}
      </Box>

      <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, pt: "3px" }}>
        {link.status === "available" && (
          <Button
            variant="contained"
            startIcon={<DownloadRounded />}
            onClick={() => run("start_download", { id: link.id })}
          >
            Descargar
          </Button>
        )}
        {link.status === "failed" && (
          <Button variant="outlined" onClick={() => run("start_download", { id: link.id })}>
            Reintentar
          </Button>
        )}
        {downloading && (
          <Button
            variant="outlined"
            startIcon={<CloseRounded />}
            onClick={() => run("cancel_download", { id: link.id })}
          >
            Cancelar
          </Button>
        )}
        {link.status === "completed" && (
          <Button
            variant="outlined"
            startIcon={<FolderOpenRounded />}
            onClick={() => run("reveal", { id: link.id })}
          >
            Mostrar
          </Button>
        )}

        {canRecheck && (
          <Tooltip title="Comprobar ahora">
            <span>
              <IconButton
                aria-label={`Comprobar ${title}`}
                disabled={link.checking}
                onClick={() => run("check_now", { id: link.id })}
              >
                <RefreshRounded
                  sx={{ fontSize: 18, animation: link.checking ? `${spin} 0.9s linear infinite` : "none" }}
                />
              </IconButton>
            </span>
          </Tooltip>
        )}
        <Tooltip title={downloading ? "Cancelar y quitar de la lista" : "Quitar de la lista"}>
          <IconButton aria-label={`Quitar ${title}`} onClick={() => run("remove_link", { id: link.id })}>
            <DeleteOutlineRounded sx={{ fontSize: 18 }} />
          </IconButton>
        </Tooltip>
      </Box>
    </Box>
  );
}

function Transfer({ link }: { link: LinkItem }) {
  const percent = link.size ? Math.min(100, (link.received / link.size) * 100) : null;
  const remaining = link.size && link.speed > 0 ? (link.size - link.received) / link.speed : 0;

  const parts = [
    percent != null ? `${Math.floor(percent)} %` : null,
    link.size ? `${bytes(link.received)} de ${bytes(link.size)}` : bytes(link.received),
    link.speed > 0 ? `${bytes(link.speed)}/s` : null,
    remaining > 0 ? `quedan ${duration(remaining)}` : null,
  ].filter(Boolean);

  return (
    <Box sx={{ mt: 1 }}>
      <LinearProgress
        variant={percent != null ? "determinate" : "indeterminate"}
        value={percent ?? undefined}
        sx={{ "& .MuiLinearProgress-bar": { transition: "transform 300ms linear" } }}
      />
      <Typography variant="caption" sx={{ display: "block", mt: 0.75, color: "text.secondary" }}>
        {parts.join(" · ")}
      </Typography>
    </Box>
  );
}
