/** @type {import('tailwindcss').Config} */
export default {
  content: ['./*.html', './src/*.css'],
  theme: {
    extend: {},
  },
  plugins: [require("daisyui")],
  daisyui: {
    themes: ["light", "dark"]
  }
}

