import type { Preview } from '@storybook/react-vite'
import { ThemeProvider } from '../src/contexts/ThemeContext'
import { LOCALE_DESCRIPTORS, type Locale } from '../src/i18n'
import { LocalizedStory } from './LocalizedStory'
import '../src/index.css'

document.documentElement.dataset.theme = 'dark'

const preview: Preview = {
  globalTypes: {
    locale: {
      description: 'Interface language',
      toolbar: {
        title: 'Language',
        icon: 'globe',
        items: LOCALE_DESCRIPTORS.map((descriptor) => ({
          value: descriptor.code,
          title: `${descriptor.nativeName} (${descriptor.code})`,
        })),
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: {
    locale: 'en',
  },
  decorators: [
    (Story, context) => (
      <LocalizedStory locale={(context.globals.locale as Locale) ?? 'en'}>
        <ThemeProvider>
          <div className="min-h-screen bg-canvas p-6 text-foreground">
            <Story />
          </div>
        </ThemeProvider>
      </LocalizedStory>
    ),
  ],
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    backgrounds: {
      default: 'Photon dark',
      values: [
        { name: 'Photon dark', value: '#0a0a0f' },
        { name: 'Photon light', value: '#f8f9fa' },
      ],
    },
  },
}

export default preview
