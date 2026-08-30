import type { Config } from 'tailwindcss'

export default {
  darkMode: ['media'],
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          'system-ui',
          '-apple-system',
          '"Hiragino Sans"',
          '"Noto Sans JP"',
          'sans-serif',
        ],
        mono: ['"SF Mono"', 'Menlo', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
} satisfies Config
