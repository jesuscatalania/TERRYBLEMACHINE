/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        "neutral-dark": {
          50: "#f7f7f8",
          100: "#ebebee",
          200: "#d4d4da",
          300: "#a9a9b2",
          400: "#72727e",
          500: "#4a4a52",
          600: "#2f2f35",
          700: "#1f1f24",
          800: "#17171b",
          900: "#0e0e11",
          950: "#06060a",
        },
        accent: {
          DEFAULT: "#e85d2d",
          50: "#fff4ef",
          100: "#ffe4d6",
          200: "#ffc4a8",
          300: "#ff9b70",
          400: "#ff7347",
          500: "#e85d2d",
          600: "#c84815",
          700: "#9f3811",
          800: "#7a2c10",
          900: "#5a2210",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "system-ui",
          "sans-serif",
        ],
        mono: [
          "IBM Plex Mono",
          "SF Mono",
          "ui-monospace",
          "Menlo",
          "Monaco",
          "Cascadia Code",
          "monospace",
        ],
        display: ["Inter", "SF Pro Display", "-apple-system", "system-ui", "sans-serif"],
      },
      fontSize: {
        "2xs": ["0.625rem", { lineHeight: "0.875rem" }],
      },
      spacing: {
        "shell-header": "3rem",
        "shell-sidebar": "15rem",
        "shell-sidebar-collapsed": "3.5rem",
        "shell-footer": "1.75rem",
        18: "4.5rem",
        128: "32rem",
      },
      borderRadius: {
        xs: "0.125rem",
      },
      letterSpacing: {
        label: "0.08em",
        "label-wide": "0.14em",
      },
      boxShadow: {
        soft: "0 1px 2px rgba(0,0,0,0.12), 0 1px 3px rgba(0,0,0,0.2)",
        elevated: "0 10px 30px rgba(0,0,0,0.4), 0 4px 10px rgba(0,0,0,0.3)",
      },
    },
  },
  plugins: [],
};
