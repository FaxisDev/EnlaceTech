import Alert from "@mui/material/Alert";
import Box from "@mui/material/Box";
import CssBaseline from "@mui/material/CssBaseline";
import Snackbar from "@mui/material/Snackbar";
import { ThemeProvider } from "@mui/material/styles";
import { useCallback, useState } from "react";

import LinksView from "./components/LinksView";
import SettingsView from "./components/SettingsView";
import Sidebar from "./components/Sidebar";
import type { View } from "./components/Sidebar";
import { useSnapshot } from "./hooks/useSnapshot";
import { call, errorText, isMac } from "./lib/bridge";
import type { Run, Say } from "./lib/types";
import { navy, theme } from "./theme";

interface Toast {
  key: number;
  text: string;
  tone: "info" | "success" | "error";
}

export default function App() {
  const snapshot = useSnapshot();
  const [view, setView] = useState<View>("links");
  const [toast, setToast] = useState<Toast | null>(null);
  const [toastOpen, setToastOpen] = useState(false);

  const say = useCallback<Say>((text, tone = "info") => {
    setToast({ key: Date.now(), text, tone });
    setToastOpen(true);
  }, []);

  const run = useCallback<Run>(
    async <T,>(cmd: string, args?: Record<string, unknown>) => {
      try {
        return await call<T>(cmd, args);
      } catch (error) {
        say(errorText(error), "error");
        return undefined;
      }
    },
    [say],
  );

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />

      {/* macOS draws the window buttons over the page; this strip is what you drag the window by. */}
      {isMac && (
        <Box
          data-tauri-drag-region
          sx={{ position: "fixed", top: 0, left: 0, right: 0, height: 32, zIndex: 10 }}
        />
      )}

      <Box sx={{ display: "grid", gridTemplateColumns: "236px minmax(0, 1fr)", height: "100vh" }}>
        <Sidebar snapshot={snapshot} view={view} onView={setView} run={run} />

        <Box
          component="main"
          sx={{
            display: "flex",
            flexDirection: "column",
            minWidth: 0,
            minHeight: 0,
            bgcolor: navy.base,
            pt: isMac ? 5.5 : 3.5,
          }}
        >
          {snapshot &&
            (view === "links" ? (
              <LinksView snapshot={snapshot} run={run} say={say} onSignIn={() => setView("settings")} />
            ) : (
              <SettingsView snapshot={snapshot} run={run} say={say} />
            ))}
        </Box>
      </Box>

      <Snackbar
        key={toast?.key}
        open={toastOpen}
        autoHideDuration={4500}
        onClose={(_, reason) => {
          if (reason !== "clickaway") setToastOpen(false);
        }}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert
          severity={toast?.tone ?? "info"}
          variant="filled"
          onClose={() => setToastOpen(false)}
          sx={{ boxShadow: "none", fontWeight: 600 }}
        >
          {toast?.text}
        </Alert>
      </Snackbar>
    </ThemeProvider>
  );
}
