/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['"Plus Jakarta Sans"', '"Noto Sans SC"', 'system-ui', '-apple-system', 'sans-serif'],
      },
      colors: {
        border: "var(--border)",
        input: "var(--input)",
        ring: "var(--ring)",
        background: "var(--background)",
        foreground: "var(--foreground)",
        primary: {
          DEFAULT: "var(--primary)",
          foreground: "var(--primary-foreground)",
        },
        secondary: {
          DEFAULT: "var(--secondary)",
          foreground: "var(--secondary-foreground)",
        },
        destructive: {
          DEFAULT: "var(--destructive)",
          foreground: "var(--destructive-foreground)",
        },
        muted: {
          DEFAULT: "var(--muted)",
          foreground: "var(--muted-foreground)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          foreground: "var(--accent-foreground)",
        },
        popover: {
          DEFAULT: "var(--popover)",
          foreground: "var(--popover-foreground)",
        },
        card: {
          DEFAULT: "var(--card)",
          foreground: "var(--card-foreground)",
        },
        success: {
          DEFAULT: "var(--color-success)",
          light: "var(--color-success-bg)",
        },
        warning: {
          DEFAULT: "var(--color-warning)",
          light: "var(--color-warning-bg)",
        },
        error: {
          DEFAULT: "var(--color-error)",
          light: "var(--color-error-bg)",
        },
        info: {
          DEFAULT: "var(--color-info)",
          light: "var(--color-info-bg)",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
        xl: "calc(var(--radius) * 1.5)",
      },
      boxShadow: {
        sm: "var(--shadow-sm)",
        md: "var(--shadow-md)",
        lg: "var(--shadow-lg)",
        xl: "var(--shadow-xl)",
      },
      keyframes: {
        "slide-in": {
          "from": { transform: "translateY(-10px)", opacity: "0" },
          "to": { transform: "translateY(0)", opacity: "1" },
        },
        "slide-in-from-top": {
          "from": { transform: "translateY(-100%)", opacity: "0" },
          "to": { transform: "translateY(0)", opacity: "1" },
        },
        "slide-in-from-bottom": {
          "from": { transform: "translateY(100%)", opacity: "0" },
          "to": { transform: "translateY(0)", opacity: "1" },
        },
        "slide-in-from-left": {
          "from": { transform: "translateX(-100%)", opacity: "0" },
          "to": { transform: "translateX(0)", opacity: "1" },
        },
        "slide-in-from-right": {
          "from": { transform: "translateX(100%)", opacity: "0" },
          "to": { transform: "translateX(0)", opacity: "1" },
        },
        "fade-in": {
          "from": { opacity: "0" },
          "to": { opacity: "1" },
        },
        "fade-in-up": {
          "from": { opacity: "0", transform: "translateY(10px)" },
          "to": { opacity: "1", transform: "translateY(0)" },
        },
        "fade-out": {
          "from": { opacity: "1" },
          "to": { opacity: "0" },
        },
        "scale-in": {
          "from": { transform: "scale(0.95)", opacity: "0" },
          "to": { transform: "scale(1)", opacity: "1" },
        },
        "scale-out": {
          "from": { transform: "scale(1)", opacity: "1" },
          "to": { transform: "scale(0.95)", opacity: "0" },
        },
        "pulse-slow": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.5" },
        },
        "spin-slow": {
          "from": { transform: "rotate(0deg)" },
          "to": { transform: "rotate(360deg)" },
        },
        "bounce-subtle": {
          "0%, 100%": { transform: "translateY(0)" },
          "50%": { transform: "translateY(-5px)" },
        },
        "shimmer": {
          "from": { backgroundPosition: "-1000px 0" },
          "to": { backgroundPosition: "1000px 0" },
        },
        "typewriter": {
          "from": { width: "0" },
          "to": { width: "100%" },
        },
        "blink": {
          "0%, 50%": { opacity: "1" },
          "51%, 100%": { opacity: "0" },
        },
      },
      animation: {
        "slide-in": "slide-in 0.2s ease-out",
        "slide-in-from-top": "slide-in-from-top 0.3s ease-out",
        "slide-in-from-bottom": "slide-in-from-bottom 0.3s ease-out",
        "slide-in-from-left": "slide-in-from-left 0.3s ease-out",
        "slide-in-from-right": "slide-in-from-right 0.3s ease-out",
        "fade-in": "fade-in 0.2s ease-out",
        "fade-in-up": "fade-in-up 0.3s ease-out",
        "fade-out": "fade-out 0.2s ease-out",
        "scale-in": "scale-in 0.2s ease-out",
        "scale-out": "scale-out 0.2s ease-out",
        "pulse-slow": "pulse-slow 3s cubic-bezier(0.4, 0, 0.6, 1) infinite",
        "spin-slow": "spin-slow 3s linear infinite",
        "bounce-subtle": "bounce-subtle 2s ease-in-out infinite",
        "shimmer": "shimmer 2s linear infinite",
        "typewriter": "typewriter 2s steps(40) infinite",
        "blink": "blink 1s step-end infinite",
      },
      // Performance: Animation delay variants for staggered animations
      animationDelay: {
        0: "0ms",
        100: "100ms",
        150: "150ms",
        200: "200ms",
        300: "300ms",
        400: "400ms",
        500: "500ms",
      },
    },
  },
  plugins: [
    require("@tailwindcss/typography")({
      theme: {
        extend: {
          colors: {},
        },
      },
    }),
  ],
}
