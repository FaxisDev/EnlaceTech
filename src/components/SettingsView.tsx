import Alert from "@mui/material/Alert";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import FormControlLabel from "@mui/material/FormControlLabel";
import Switch from "@mui/material/Switch";
import TextField from "@mui/material/TextField";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Typography from "@mui/material/Typography";
import FolderOpenRounded from "@mui/icons-material/FolderOpenRounded";
import LogoutRounded from "@mui/icons-material/LogoutRounded";
import NotificationsNoneRounded from "@mui/icons-material/NotificationsNoneRounded";
import { useState } from "react";
import type { FormEvent, ReactNode } from "react";

import { call, errorText, isMac } from "../lib/bridge";
import { bytes, longDate } from "../lib/format";
import type { Run, Say, Snapshot } from "../lib/types";
import { mono, navy, tones } from "../theme";

const INTERVALS = [5, 10, 15, 30, 60];

/** Where the app's icon stays while the window is closed, as each system calls it. */
const TRAY_PLACE = isMac ? "la barra de menús" : "la bandeja del sistema";

interface Props {
  snapshot: Snapshot;
  run: Run;
}

export default function SettingsView({ snapshot, run, say }: Props & { say: Say }) {
  return (
    <>
      <Box sx={{ px: 4, pb: 1 }}>
        <Typography variant="h5" component="h1">
          Ajustes
        </Typography>
        <Typography sx={{ color: "text.secondary", mt: 0.25 }}>
          Tu cuenta, dónde se guardan los archivos y cada cuánto se comprueban los enlaces.
        </Typography>
      </Box>

      <Box sx={{ flex: 1, minHeight: 0, overflowY: "auto", px: 4, pb: 4 }}>
        <Section
          title="Cuenta de Rapidgator"
          text="EnlaceTech usa la API oficial de Rapidgator, que necesita que inicies sesión con tu cuenta."
        >
          {!snapshot.ready ? (
            <Typography sx={{ color: "text.secondary" }}>Recuperando tu sesión…</Typography>
          ) : snapshot.signedIn ? (
            <AccountCard snapshot={snapshot} run={run} />
          ) : (
            <LoginForm snapshot={snapshot} />
          )}
        </Section>

        <Section
          title="Carpeta de descargas"
          text="Los archivos se guardan aquí. Si ya existe uno con el mismo nombre, no se sobrescribe."
        >
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              gap: 1.25,
              bgcolor: navy.deep,
              border: `1px solid ${navy.line}`,
              borderRadius: "8px",
              px: 1.5,
              minHeight: 40,
            }}
          >
            <FolderOpenRounded sx={{ fontSize: 18, color: "text.disabled" }} />
            <Typography
              noWrap
              title={snapshot.downloadDir}
              sx={{ fontFamily: mono, fontSize: 12.5, flex: 1, minWidth: 0, direction: "rtl", textAlign: "left" }}
            >
              {/* `direction: rtl` keeps the end of a long path visible; the mark keeps its order intact. */}
              &lrm;{snapshot.downloadDir}
            </Typography>
          </Box>
          <Box sx={{ display: "flex", flexWrap: "wrap", gap: 1, mt: 1.5 }}>
            <Button variant="contained" onClick={() => run("pick_download_dir")}>
              Cambiar carpeta…
            </Button>
            <Button variant="outlined" onClick={() => run("open_download_dir")}>
              Abrir
            </Button>
            <Button
              variant="outlined"
              disabled={snapshot.defaultDir}
              onClick={() => run("reset_download_dir")}
            >
              Usar Descargas del sistema
            </Button>
          </Box>
          {snapshot.defaultDir && (
            <Typography variant="caption" sx={{ display: "block", mt: 1.25, color: "text.disabled" }}>
              Es la carpeta Descargas de tu sistema, la opción por defecto.
            </Typography>
          )}
        </Section>

        <Section
          title="Comprobación automática"
          text="Cada cuánto pregunta EnlaceTech a Rapidgator si tus enlaces ya se pueden descargar."
        >
          <ToggleButtonGroup
            exclusive
            size="small"
            value={snapshot.intervalMinutes}
            onChange={(_, minutes: number | null) => {
              if (minutes != null) void run("set_interval", { minutes });
            }}
            aria-label="Intervalo de comprobación"
          >
            {INTERVALS.map((minutes) => (
              <ToggleButton key={minutes} value={minutes}>
                {minutes < 60 ? `${minutes} min` : "1 h"}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
          <Typography variant="caption" sx={{ display: "block", mt: 1.25, color: "text.disabled", maxWidth: 440 }}>
            Rapidgator pide a las aplicaciones no hacer más llamadas de las necesarias, por eso el mínimo son
            5 minutos. Siempre puedes comprobar a mano cuando quieras.
          </Typography>
        </Section>

        <Section
          title="En segundo plano"
          text={`Qué pasa al cerrar la ventana. El icono de EnlaceTech se queda en ${TRAY_PLACE}.`}
        >
          <FormControlLabel
            sx={{ ml: -0.75, alignItems: "flex-start", "& .MuiFormControlLabel-label": { pt: "7px" } }}
            control={
              <Switch
                checked={snapshot.keepInTray}
                onChange={(e) => void run("set_keep_in_tray", { enabled: e.target.checked })}
              />
            }
            label={
              <Box>
                <Typography variant="body2" sx={{ fontWeight: 600 }}>
                  Seguir comprobando al cerrar la ventana
                </Typography>
                <Typography variant="caption" sx={{ color: "text.secondary", display: "block" }}>
                  {snapshot.keepInTray
                    ? "La ventana se oculta y la app sigue abierta. Para cerrarla del todo, elige Salir en el menú de su icono."
                    : "Al cerrar la ventana, la app se cierra y deja de comprobar."}
                </Typography>
              </Box>
            }
          />
          <Typography variant="caption" sx={{ display: "block", mt: 1.25, color: "text.disabled", maxWidth: 440 }}>
            Desde ese icono puedes abrir la ventana, lanzar una comprobación o salir.
          </Typography>
        </Section>

        <Section
          title="Avisos"
          text="Una notificación del sistema cuando un enlace que estaba en espera ya se puede descargar."
          last
        >
          <FormControlLabel
            sx={{ ml: -0.75, alignItems: "flex-start", "& .MuiFormControlLabel-label": { pt: "7px" } }}
            control={
              <Switch
                checked={snapshot.notifications}
                onChange={(e) => void run("set_notifications", { enabled: e.target.checked })}
              />
            }
            label={
              <Box>
                <Typography variant="body2" sx={{ fontWeight: 600 }}>
                  Avisar cuando un enlace quede listo
                </Typography>
                <Typography variant="caption" sx={{ color: "text.secondary", display: "block" }}>
                  Solo si EnlaceTech no está en primer plano: si la estás mirando, ya lo ves en la lista.
                </Typography>
              </Box>
            }
          />
          <Box sx={{ mt: 1.5 }}>
            <Button
              variant="outlined"
              startIcon={<NotificationsNoneRounded />}
              onClick={async () => {
                if ((await run<null>("test_notification")) !== undefined) say("Aviso de prueba enviado.", "success");
              }}
            >
              Enviar aviso de prueba
            </Button>
          </Box>
          <Typography variant="caption" sx={{ display: "block", mt: 1.25, color: "text.disabled", maxWidth: 440 }}>
            Si el aviso de prueba no aparece, revisa los permisos de notificaciones de tu sistema.
          </Typography>
        </Section>
      </Box>
    </>
  );
}

function Section(props: { title: string; text: string; children: ReactNode; last?: boolean }) {
  const { title, text, children, last } = props;
  return (
    <Box
      component="section"
      sx={{
        display: "grid",
        gridTemplateColumns: { xs: "1fr", md: "250px minmax(0, 1fr)" },
        gap: { xs: 1.5, md: 4 },
        py: 3,
        borderBottom: last ? "none" : `1px solid ${navy.lineSoft}`,
      }}
    >
      <Box>
        <Typography variant="h6" component="h2">
          {title}
        </Typography>
        <Typography variant="body2" sx={{ color: "text.secondary", mt: 0.5, textWrap: "pretty" }}>
          {text}
        </Typography>
      </Box>
      <Box sx={{ minWidth: 0, maxWidth: 520 }}>{children}</Box>
    </Box>
  );
}

function AccountCard({ snapshot, run }: Props) {
  const account = snapshot.account;
  if (!account) return null;

  const facts: Array<[string, string]> = [["Plan", account.isPremium ? "Premium" : "Gratuita"]];
  if (account.isPremium && account.premiumEnd) facts.push(["Premium hasta", longDate(account.premiumEnd)]);
  if (account.trafficLeft != null) {
    facts.push([
      "Tráfico disponible",
      account.trafficTotal != null
        ? `${bytes(account.trafficLeft)} de ${bytes(account.trafficTotal)}`
        : bytes(account.trafficLeft),
    ]);
  }

  return (
    <Box>
      <Typography sx={{ fontSize: 14.5, fontWeight: 600 }}>{account.email}</Typography>

      <Box component="dl" sx={{ m: 0, mt: 1.5, display: "grid", gridTemplateColumns: "auto 1fr", columnGap: 3, rowGap: 0.75 }}>
        {facts.map(([name, value]) => (
          <Box key={name} sx={{ display: "contents" }}>
            <Typography component="dt" variant="body2" sx={{ color: "text.secondary" }}>
              {name}
            </Typography>
            <Typography
              component="dd"
              variant="body2"
              sx={{ m: 0, fontWeight: 600, color: name === "Plan" && account.isPremium ? tones.ok.fg : "text.primary" }}
            >
              {value}
            </Typography>
          </Box>
        ))}
      </Box>

      {!account.isPremium && (
        <Alert severity="info" variant="outlined" sx={{ mt: 2, borderColor: navy.line, color: "text.primary" }}>
          Con una cuenta gratuita, Rapidgator puede rechazar las descargas hechas por su API. Si pasa, verás
          su mensaje junto a cada enlace.
        </Alert>
      )}

      <Button
        variant="outlined"
        startIcon={<LogoutRounded />}
        sx={{ mt: 2 }}
        onClick={() => run("logout")}
      >
        Cerrar sesión
      </Button>
      <Typography variant="caption" sx={{ display: "block", mt: 1.25, color: "text.disabled" }}>
        {snapshot.rememberPassword
          ? "La sesión y la contraseña están en el llavero de tu sistema, no en archivos de EnlaceTech."
          : "La sesión está en el llavero de tu sistema. Cuando caduque tendrás que volver a entrar."}
      </Typography>
    </Box>
  );
}

function LoginForm({ snapshot }: { snapshot: Snapshot }) {
  const [email, setEmail] = useState(snapshot.account?.email ?? "");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [remember, setRemember] = useState(snapshot.rememberPassword);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await call("login", { email, password, code: code.trim() || null, remember });
      // On success the backend's snapshot swaps this form for the account card.
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    } finally {
      setPassword("");
    }
  }

  return (
    <Box component="form" onSubmit={submit} sx={{ display: "grid", gap: 1.5 }}>
      <TextField
        label="Correo"
        type="email"
        size="small"
        required
        autoComplete="username"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        slotProps={{ htmlInput: { spellCheck: false, autoCapitalize: "off" } }}
      />
      <TextField
        label="Contraseña"
        type="password"
        size="small"
        required
        autoComplete="current-password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
      />
      <TextField
        label="Código de verificación (solo si usas 2FA)"
        size="small"
        autoComplete="one-time-code"
        value={code}
        onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 8))}
        slotProps={{ htmlInput: { inputMode: "numeric" } }}
      />

      <FormControlLabel
        sx={{ ml: -0.75, alignItems: "flex-start", "& .MuiFormControlLabel-label": { pt: "7px" } }}
        control={<Switch checked={remember} onChange={(e) => setRemember(e.target.checked)} />}
        label={
          <Box>
            <Typography variant="body2" sx={{ fontWeight: 600 }}>
              Recordar la contraseña
            </Typography>
            <Typography variant="caption" sx={{ color: "text.secondary", display: "block" }}>
              Se guarda en el llavero del sistema para renovar la sesión sola cuando caduque.
            </Typography>
          </Box>
        }
      />

      {error && (
        <Alert severity="error" variant="outlined" sx={{ color: "text.primary" }}>
          {error}
        </Alert>
      )}

      <Box>
        <Button type="submit" variant="contained" disabled={busy || !email.trim() || !password}>
          {busy ? "Entrando…" : "Iniciar sesión"}
        </Button>
      </Box>
      <Typography variant="caption" sx={{ color: "text.disabled" }}>
        Tus datos solo se envían a rapidgator.net, por conexión cifrada.
      </Typography>
    </Box>
  );
}
