import { defineConfig } from 'vite'

export default defineConfig({
  publicDir: false,
  plugins: [
    {
      name: 'public-docs-routes',
      generateBundle() {
        this.emitFile({
          type: 'asset',
          fileName: '_routes.json',
          source: JSON.stringify({
            version: 1,
            include: ['/public/*', '/robots.txt'],
            exclude: [],
          }),
        })
      },
    },
  ],
  build: {
    emptyOutDir: false,
    lib: {
      entry: 'workers/public-docs/index.ts',
      formats: ['es'],
      fileName: () => '_worker.js',
    },
  },
})
