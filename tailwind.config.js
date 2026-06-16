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
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          hover: "hsl(var(--destructive-hover))",
        },
        ring: "hsl(var(--ring))",
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
        sans: ["var(--font-sans)", "Noto Sans CJK SC", "system-ui", "-apple-system", "sans-serif"],
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
      transitionDuration: {
        120: "120ms",
      },
      boxShadow: {
        1: "0 1px 2px rgba(0, 0, 0, 0.06), 0 1px 3px rgba(0, 0, 0, 0.05)",
        2: "0 4px 12px rgba(0, 0, 0, 0.10)",
        3: "0 12px 32px rgba(0, 0, 0, 0.16)",
      },
      zIndex: {
        sticky: "40",
        dropdown: "50",
        modal: "100",
        toast: "110",
      },
    },
  },
  plugins: [],
};
