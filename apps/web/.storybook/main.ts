import type { StorybookConfig } from '@storybook/nextjs'

import { dirname, join, resolve } from 'node:path'

/**
 * This function is used to resolve the absolute path of a package.
 * It is needed in projects that use Yarn PnP or are set up within a monorepo.
 */
function getAbsolutePath(value: string) {
	return dirname(require.resolve(join(value, 'package.json')))
}
const config: StorybookConfig = {
	stories: ['../src/**/*.mdx', '../src/**/*stories.@(js|jsx|mjs|ts|tsx)'],

	addons: [
		getAbsolutePath('@storybook/addon-onboarding'),
		getAbsolutePath('@storybook/addon-links'),
		getAbsolutePath('@storybook/addon-essentials'),
		getAbsolutePath('@chromatic-com/storybook'),
		getAbsolutePath('@storybook/addon-interactions'),
		getAbsolutePath('@storybook/addon-mdx-gfm'),
		'msw-storybook-addon',
	],

	framework: {
		name: getAbsolutePath('@storybook/nextjs'),
		options: {},
	},

	staticDirs: ['../public'],

	docs: {},

	typescript: {
		reactDocgen: 'react-docgen-typescript',
	},
	features: {
		experimentalRSC: true,
	},

	webpackFinal: async (config) => {
		const updatedConfig = { ...config }
		updatedConfig.resolve = updatedConfig.resolve || {}
		updatedConfig.resolve.fallback = {
			...updatedConfig.resolve.fallback,
			crypto: require.resolve('crypto-browserify'),
			stream: require.resolve('stream-browserify'),
			buffer: require.resolve('buffer'),
			util: require.resolve('util'),
			events: require.resolve('events'),
			process: require.resolve('process/browser'),
			fs: false,
			path: require.resolve('path-browserify'),
			os: require.resolve('os-browserify/browser'),
		}

		updatedConfig.resolve.alias = {
			...updatedConfig.resolve.alias,
			'node:crypto': require.resolve('crypto-browserify'),
			'node:buffer': require.resolve('buffer'),
			'node:stream': require.resolve('stream-browserify'),
			'node:util': require.resolve('util'),
			'node:events': require.resolve('events'),
			'node:process': require.resolve('process/browser'),
			'node:path': require.resolve('path-browserify'),
			'node:os': require.resolve('os-browserify/browser'),
			// Use built dist files for monorepo packages
			// Note: packages/react must be built before running storybook build
			'@tachyon-apps/react': resolve(__dirname, '../../../packages/react/dist'),
		}

		updatedConfig.module = updatedConfig.module || {}
		updatedConfig.module.rules = updatedConfig.module.rules || []
		updatedConfig.module.rules.push({
			test: /node:/,
			resolve: {
				fullySpecified: false,
			},
		})

		if (updatedConfig.plugins) {
			const webpack = require('webpack')
			updatedConfig.plugins.push(
				new webpack.ProvidePlugin({
					Buffer: ['buffer', 'Buffer'],
					process: 'process/browser',
				}),
				new webpack.NormalModuleReplacementPlugin(/^node:/, (resource) => {
					const replacements: Record<string, string> = {
						'node:crypto': 'crypto-browserify',
						'node:buffer': 'buffer',
						'node:stream': 'stream-browserify',
						'node:util': 'util',
						'node:events': 'events',
						'node:process': 'process/browser',
						'node:path': 'path-browserify',
						'node:os': 'os-browserify/browser',
					}

					resource.request =
						replacements[resource.request] || resource.request
				}),
			)
		}

		return updatedConfig
	},
}
export default config
