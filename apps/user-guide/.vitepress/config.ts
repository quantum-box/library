import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Library ユーザーガイド',
  description: 'Library の API キー発行と API の使い方',
  lang: 'ja-JP',
  cleanUrls: true,
  themeConfig: {
    nav: [
      { text: 'ホーム', link: '/' },
      { text: 'API', link: '/api/getting-started' },
    ],
    sidebar: [
      {
        text: 'API を使う',
        items: [
          { text: 'はじめに', link: '/api/getting-started' },
          { text: 'API キーの発行と失効', link: '/api/api-keys' },
          { text: 'REST API', link: '/api/rest' },
          { text: 'GraphQL API', link: '/api/graphql' },
        ],
      },
    ],
    outline: { label: '目次', level: [2, 3] },
    docFooter: { prev: '前へ', next: '次へ' },
  },
})
