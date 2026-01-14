/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Core background palette
        bg: {
          app: '#06080f',
          sidebar: '#0b1322',
          panel: '#0b1222',
          active: '#14243a',
          hover: '#1a2b45',
        },
        // Border tones
        border: {
          DEFAULT: 'rgba(96, 168, 255, 0.28)',
          light: 'rgba(96, 168, 255, 0.45)',
        },
        // Text tones
        text: {
          main: '#e9f3ff',
          muted: '#9fb4d3',
          dim: '#566a86',
          faint: '#3f506a',
        },
        // Accent tones
        accent: {
          DEFAULT: '#4eeaff',
          hover: '#6df1ff',
          pink: '#ff5edb',
          lime: '#7dff9d',
          glow: 'rgba(78, 234, 255, 0.2)',
        },
        // Semantic tones
        status: {
          success: '#7dff7a',
          warning: '#ffd166',
          error: '#ff6b6b',
          info: '#5aa7ff',
        }
      },
      fontFamily: {
        sans: ['Rajdhani', 'system-ui', 'sans-serif'],
        display: ['Orbitron', 'Rajdhani', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      boxShadow: {
        glow: '0 0 22px -6px var(--tw-shadow-color)',
        frame: '0 0 0 1px rgba(99, 159, 255, 0.35), 0 0 30px rgba(54, 246, 255, 0.12)',
      }
    },
  },
  plugins: [],
}
