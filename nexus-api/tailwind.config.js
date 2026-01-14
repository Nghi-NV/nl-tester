/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      colors: {
        glass: "rgba(255, 255, 255, 0.05)",
        glassHover: "rgba(255, 255, 255, 0.1)",
        borderGlass: "rgba(255, 255, 255, 0.1)",
      }
    },
  },
  plugins: [],
}
