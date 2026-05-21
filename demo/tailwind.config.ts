import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./lib/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Bloomberg-inspired palette
        ink: "#000000",
        "ink-soft": "#1a1a1a",
        muted: "#6b7280",
        line: "#e5e7eb",
        "line-strong": "#d1d5db",
        canvas: "#ffffff",
        "canvas-alt": "#fafafa",
        // Status colors — pale Bloomberg-green for gains, red for losses
        gain: "#10b981",
        "gain-soft": "#a7f3d0",
        loss: "#ef4444",
        "loss-soft": "#fecaca",
        accent: "#f59e0b", // Bloomberg's signature orange
      },
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "monospace",
        ],
      },
      fontSize: {
        "hero": ["3.5rem", { lineHeight: "1.05", letterSpacing: "-0.02em", fontWeight: "800" }],
      },
    },
  },
  plugins: [],
};

export default config;
