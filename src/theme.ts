import { createTheme, keyframes } from "@mui/material/styles";

/** The navy scale the whole app is painted with, from deepest to lightest. */
export const navy = {
  deep: "#081328",
  base: "#0B1930",
  surface: "#10223F",
  raised: "#162C50",
  line: "#1D3459",
  lineSoft: "#152847",
} as const;

/** Colour pairs for states: a foreground and a soft wash to put behind it. */
export const tones = {
  ok: { fg: "#3FD2A0", wash: "rgba(63, 210, 160, 0.13)" },
  warn: { fg: "#F2B65C", wash: "rgba(242, 182, 92, 0.13)" },
  bad: { fg: "#FF7F89", wash: "rgba(255, 127, 137, 0.13)" },
  info: { fg: "#5B97FF", wash: "rgba(79, 143, 255, 0.16)" },
  muted: { fg: "#8497B8", wash: "rgba(132, 151, 184, 0.12)" },
} as const;

export type Tone = keyof typeof tones;

export const spin = keyframes`
  to { transform: rotate(360deg); }
`;

const fontFamily = [
  "-apple-system",
  "BlinkMacSystemFont",
  '"SF Pro Text"',
  '"Segoe UI Variable Text"',
  '"Segoe UI"',
  "Inter",
  "Roboto",
  '"Helvetica Neue"',
  "sans-serif",
].join(",");

export const mono = 'ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, monospace';

export const theme = createTheme({
  palette: {
    mode: "dark",
    primary: { main: "#4F8FFF", light: "#74A7FF", dark: "#3D78E0", contrastText: "#06122A" },
    success: { main: tones.ok.fg, contrastText: "#06221A" },
    warning: { main: tones.warn.fg, contrastText: "#2A1B02" },
    error: { main: tones.bad.fg, contrastText: "#2C0509" },
    info: { main: tones.info.fg, contrastText: "#06122A" },
    background: { default: navy.base, paper: navy.surface },
    text: { primary: "#E9EFF9", secondary: "#A3B4D0", disabled: "#6A7FA3" },
    divider: navy.line,
    action: {
      hover: "rgba(120, 160, 230, 0.08)",
      selected: "rgba(79, 143, 255, 0.14)",
      disabled: "#5C7198",
      disabledBackground: "rgba(120, 160, 230, 0.10)",
    },
  },
  shape: { borderRadius: 10 },
  typography: {
    fontFamily,
    fontSize: 13,
    h5: { fontSize: 22, fontWeight: 650, letterSpacing: "-0.01em" },
    h6: { fontSize: 15, fontWeight: 620, letterSpacing: "-0.005em" },
    subtitle1: { fontSize: 14, fontWeight: 580, lineHeight: 1.35 },
    body1: { fontSize: 13.5, lineHeight: 1.5 },
    body2: { fontSize: 12.5, lineHeight: 1.5 },
    caption: { fontSize: 11.5, lineHeight: 1.45 },
    overline: { fontSize: 10.5, fontWeight: 650, letterSpacing: "0.09em", lineHeight: 1.6 },
    button: { textTransform: "none", fontWeight: 600, fontSize: 13 },
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        "html, body, #root": { height: "100%" },
        body: {
          overflow: "hidden",
          cursor: "default",
          userSelect: "none",
          WebkitUserSelect: "none",
          WebkitFontSmoothing: "antialiased",
          fontVariantNumeric: "tabular-nums",
        },
        "input, textarea": { userSelect: "text", WebkitUserSelect: "text" },
        "*::-webkit-scrollbar": { width: 12, height: 12 },
        "*::-webkit-scrollbar-track": { background: "transparent" },
        "*::-webkit-scrollbar-thumb": {
          backgroundColor: "#213B66",
          borderRadius: 8,
          border: "4px solid transparent",
          backgroundClip: "content-box",
        },
        "@media (prefers-reduced-motion: reduce)": {
          "*": { animationDuration: "0.001ms !important", transitionDuration: "0.001ms !important" },
        },
      },
    },
    // Flat: no ripples, no shadows, no gradients.
    MuiButtonBase: { defaultProps: { disableRipple: true } },
    MuiPaper: {
      defaultProps: { elevation: 0 },
      styleOverrides: { root: { backgroundImage: "none" } },
    },
    MuiButton: {
      defaultProps: { disableElevation: true },
      styleOverrides: {
        root: { borderRadius: 8, minHeight: 34, paddingInline: 14, whiteSpace: "nowrap" },
        outlined: {
          borderColor: navy.line,
          color: "#E9EFF9",
          "&:hover": { borderColor: "#2B4878", backgroundColor: "rgba(120, 160, 230, 0.08)" },
        },
      },
    },
    MuiIconButton: {
      styleOverrides: {
        root: {
          borderRadius: 8,
          color: "#7E93B7",
          "&:hover": { color: "#E9EFF9", backgroundColor: "rgba(120, 160, 230, 0.10)" },
        },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: {
          backgroundColor: navy.deep,
          borderRadius: 8,
          "& .MuiOutlinedInput-notchedOutline": { borderColor: navy.line },
          "&:hover .MuiOutlinedInput-notchedOutline": { borderColor: "#2B4878" },
        },
      },
    },
    MuiLinearProgress: {
      styleOverrides: {
        root: { height: 4, borderRadius: 2, backgroundColor: navy.line },
        bar: { borderRadius: 2 },
      },
    },
    MuiTooltip: {
      defaultProps: { enterDelay: 500, arrow: false },
      styleOverrides: {
        tooltip: {
          backgroundColor: "#1B3460",
          color: "#E9EFF9",
          fontSize: 11.5,
          fontWeight: 500,
          borderRadius: 6,
          padding: "5px 9px",
        },
      },
    },
    MuiToggleButton: {
      styleOverrides: {
        root: {
          textTransform: "none",
          fontWeight: 600,
          paddingInline: 14,
          paddingBlock: 6,
          borderColor: navy.line,
          color: "#A3B4D0",
          "&.Mui-selected": { color: "#06122A", backgroundColor: "#4F8FFF" },
          "&.Mui-selected:hover": { backgroundColor: "#74A7FF" },
        },
      },
    },
    MuiAlert: {
      styleOverrides: {
        root: { borderRadius: 10, alignItems: "center", fontSize: 12.5 },
      },
    },
  },
});
