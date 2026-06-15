/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        background: "hsl(var(--background))",
        sidebar: "hsl(var(--sidebar))",
        elevated: "hsl(var(--elevated))",
        "elevated-hover": "hsl(var(--elevated-hover))",
        border: "hsl(var(--border))",
        foreground: "hsl(var(--foreground))",
        "muted-foreground": "hsl(var(--muted-foreground))",
        "faint-foreground": "hsl(var(--faint-foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          hover: "hsl(var(--primary-hover))",
        },
        "table-even": "hsl(var(--table-even))",
        "table-odd": "hsl(var(--table-odd))",
        overlay: "hsl(var(--overlay))",
      },
      borderRadius: {
        sm: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
        xl: "var(--radius-xl)",
        full: "var(--radius-full)",
      },
      fontFamily: {
        sans: [
          "Noto Sans CJK SC",
          "system-ui",
          "-apple-system",
          "sans-serif",
        ],
        mono: ["ui-monospace", "monospace"],
      },
      fontSize: {
        caption: ["11px", { lineHeight: "1.2", fontWeight: "500" }],
        "body-sm": ["13px", { lineHeight: "1.4", fontWeight: "400" }],
        body: ["14px", { lineHeight: "1.4", fontWeight: "400" }],
        "body-md": ["14px", { lineHeight: "1.4", fontWeight: "600" }],
        "title-sm": ["16px", { lineHeight: "1.3", fontWeight: "600" }],
        title: ["18px", { lineHeight: "1.3", fontWeight: "700" }],
        display: ["24px", { lineHeight: "1.2", fontWeight: "700" }],
      },
      spacing: {
        sp1: "4px",
        sp2: "8px",
        sp3: "12px",
        sp4: "16px",
        sp6: "24px",
        sp8: "32px",
      },
    },
  },
  plugins: [],
};
