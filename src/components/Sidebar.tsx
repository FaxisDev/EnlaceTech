import Avatar from "@mui/material/Avatar";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import ButtonBase from "@mui/material/ButtonBase";
import LinearProgress from "@mui/material/LinearProgress";
import Typography from "@mui/material/Typography";
import LinkRounded from "@mui/icons-material/LinkRounded";
import RefreshRounded from "@mui/icons-material/RefreshRounded";
import TuneRounded from "@mui/icons-material/TuneRounded";
import type { ReactNode } from "react";

import { useNow } from "../hooks/useSnapshot";
import { isMac } from "../lib/bridge";
import { ago, countdown } from "../lib/format";
import type { Run, Snapshot } from "../lib/types";
import { navy, spin, tones } from "../theme";
import { Mark } from "./Brand";

export type View = "links" | "settings";

interface Props {
  snapshot: Snapshot | null;
  view: View;
  onView: (view: View) => void;
  run: Run;
}

export default function Sidebar({ snapshot, view, onView, run }: Props) {
  const ready = snapshot?.links.filter((l) => l.status === "available").length ?? 0;

  return (
    <Box
      component="aside"
      sx={{
        bgcolor: navy.deep,
        borderRight: `1px solid ${navy.lineSoft}`,
        display: "flex",
        flexDirection: "column",
        // Leave room for the window buttons that macOS draws over the content.
        pt: isMac ? 7 : 2.5,
        px: 1.5,
        pb: 1.5,
        minHeight: 0,
      }}
    >
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.1, px: 1.25, mb: 3 }}>
        <Mark size={24} />
        <Typography sx={{ fontSize: 16, fontWeight: 680, letterSpacing: "-0.01em" }}>EnlaceTech</Typography>
      </Box>

      <Box component="nav" sx={{ display: "grid", gap: 0.5 }}>
        <NavItem
          icon={<LinkRounded sx={{ fontSize: 18 }} />}
          label="Enlaces"
          active={view === "links"}
          onClick={() => onView("links")}
          badge={ready > 0 ? ready : undefined}
        />
        <NavItem
          icon={<TuneRounded sx={{ fontSize: 18 }} />}
          label="Ajustes"
          active={view === "settings"}
          onClick={() => onView("settings")}
          dot={snapshot?.ready && !snapshot.signedIn}
        />
      </Box>

      <Box sx={{ flex: 1 }} />

      {snapshot && <Pulse snapshot={snapshot} run={run} onSignIn={() => onView("settings")} />}
    </Box>
  );
}

function NavItem(props: {
  icon: ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
  badge?: number;
  dot?: boolean;
}) {
  const { icon, label, active, onClick, badge, dot } = props;
  return (
    <ButtonBase
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      sx={{
        justifyContent: "flex-start",
        gap: 1.25,
        height: 36,
        px: 1.25,
        borderRadius: "8px",
        fontSize: 13.5,
        fontWeight: active ? 620 : 520,
        color: active ? "text.primary" : "text.secondary",
        bgcolor: active ? navy.surface : "transparent",
        transition: "background-color 120ms, color 120ms",
        "&:hover": { bgcolor: active ? navy.surface : "rgba(120, 160, 230, 0.07)", color: "text.primary" },
        "&.Mui-focusVisible": { outline: `2px solid ${tones.info.fg}`, outlineOffset: 1 },
      }}
    >
      <Box sx={{ display: "flex", color: active ? "primary.main" : "inherit" }}>{icon}</Box>
      <Box sx={{ flex: 1, textAlign: "left" }}>{label}</Box>
      {badge !== undefined && (
        <Box
          sx={{
            minWidth: 20,
            height: 20,
            px: 0.75,
            borderRadius: "10px",
            bgcolor: tones.ok.wash,
            color: tones.ok.fg,
            fontSize: 11.5,
            fontWeight: 700,
            display: "grid",
            placeItems: "center",
          }}
        >
          {badge}
        </Box>
      )}
      {dot && <Box sx={{ width: 7, height: 7, borderRadius: "50%", bgcolor: tones.warn.fg }} />}
    </ButtonBase>
  );
}

/** The countdown to the next automatic check, and who is signed in. */
function Pulse({ snapshot, run, onSignIn }: { snapshot: Snapshot; run: Run; onSignIn: () => void }) {
  const now = useNow(1000);
  const { signedIn, checking, account, ready } = snapshot;

  return (
    <Box>
      <Box sx={{ bgcolor: navy.surface, borderRadius: "12px", p: 1.75 }}>
        <Typography variant="overline" sx={{ color: "text.disabled", display: "block" }}>
          {checking ? "Comprobando" : "Próxima comprobación"}
        </Typography>

        {checking ? (
          <Box sx={{ height: 36, display: "flex", alignItems: "center" }}>
            <LinearProgress sx={{ width: "100%" }} />
          </Box>
        ) : (
          <Typography
            sx={{
              fontSize: 26,
              fontWeight: 620,
              lineHeight: "36px",
              letterSpacing: "-0.02em",
              color: signedIn ? "text.primary" : "text.disabled",
            }}
          >
            {signedIn ? countdown(snapshot.nextCheck, now) : "—:——"}
          </Typography>
        )}

        <Typography variant="caption" sx={{ color: "text.secondary", display: "block", mb: 1.5 }}>
          {!ready
            ? "Recuperando tu sesión…"
            : signedIn
              ? `Última: ${ago(snapshot.lastCheck, now)}`
              : "Inicia sesión para comprobar"}
        </Typography>

        {signedIn ? (
          <Button
            fullWidth
            variant="outlined"
            disabled={checking}
            onClick={() => run("check_now", { id: null })}
            startIcon={
              <RefreshRounded
                sx={{ fontSize: "17px !important", animation: checking ? `${spin} 0.9s linear infinite` : "none" }}
              />
            }
          >
            Comprobar ahora
          </Button>
        ) : (
          <Button fullWidth variant="contained" disabled={!ready} onClick={onSignIn}>
            Iniciar sesión
          </Button>
        )}
      </Box>

      {signedIn && account && (
        <Box sx={{ display: "flex", alignItems: "center", gap: 1.25, px: 0.75, pt: 1.75, pb: 0.25 }}>
          <Avatar
            sx={{
              width: 30,
              height: 30,
              fontSize: 13,
              fontWeight: 700,
              bgcolor: navy.raised,
              color: "text.primary",
            }}
          >
            {account.email.charAt(0).toUpperCase() || "·"}
          </Avatar>
          <Box sx={{ minWidth: 0 }}>
            <Typography noWrap sx={{ fontSize: 12.5, fontWeight: 560 }} title={account.email}>
              {account.email}
            </Typography>
            <Typography variant="caption" sx={{ color: account.isPremium ? tones.ok.fg : "text.disabled" }}>
              {account.isPremium ? "Premium" : "Cuenta gratuita"}
            </Typography>
          </Box>
        </Box>
      )}
    </Box>
  );
}
