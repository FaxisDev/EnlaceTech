import Box from "@mui/material/Box";

/** The EnlaceTech mark: two links of a chain. `size` is the side of its square, in pixels. */
export function Mark({ size = 22, faint = false }: { size?: number; faint?: boolean }) {
  return (
    <Box
      component="svg"
      viewBox="0 0 24 24"
      aria-hidden
      sx={{ width: size, height: size, display: "block", flex: "none", opacity: faint ? 0.5 : 1 }}
    >
      <g transform="rotate(-45 12 12)" fill="none" strokeWidth="2.3">
        <rect x="2.5" y="8.5" width="11" height="7" rx="3.5" stroke="#2E5BA8" />
        <rect x="10.5" y="8.5" width="11" height="7" rx="3.5" stroke="#5B9BFF" />
      </g>
    </Box>
  );
}
