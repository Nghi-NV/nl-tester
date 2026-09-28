/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      colors: {
        slate: {
          50: '#f5f5f5',
          100: '#e5e5e5',
          200: '#cccccc',
          300: '#b7b7b7',
          400: '#9d9d9d',
          500: '#858585',
          600: '#6e6e6e',
          700: '#3c3c3c',
          800: '#2d2d2d',
          900: '#252526',
          950: '#1f1f1f',
        },
        cyan: {
          50: '#eff8ff',
          100: '#d9efff',
          200: '#b8e1ff',
          300: '#8ccaff',
          400: '#4daafc',
          500: '#3794ff',
          600: '#0078d4',
          700: '#0063b1',
          800: '#004e8a',
          900: '#003d6b',
          950: '#082f49',
        },
        emerald: {
          400: '#89d185',
          500: '#6a9955',
          600: '#4b7a3d',
          950: '#1e2b1b',
        },
        rose: {
          300: '#ffaaaa',
          400: '#f48771',
          500: '#f14c4c',
          600: '#d83b3b',
          700: '#b72f2f',
          900: '#5b2424',
          950: '#3b2222',
        },
        glass: "rgba(255, 255, 255, 0.02)",
        glassHover: "rgba(255, 255, 255, 0.06)",
        borderGlass: "rgba(255, 255, 255, 0.07)",
      }
    },
  },
  plugins: [],
}
