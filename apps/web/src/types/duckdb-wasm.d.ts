declare module '@duckdb/duckdb-wasm' {
	export class ConsoleLogger {
		constructor(level?: 'error' | 'info' | 'debug')
	}

	export class AsyncDuckDBConnection {
		query: <T extends Record<string, unknown> = Record<string, unknown>>(
			sql: string,
		) => Promise<{ toArray: () => T[] }>
		close: () => Promise<void>
	}

	export enum DuckDBDataProtocol {
		HTTP = 1,
		BROWSER_FILEREADER = 2,
	}

	export class AsyncDuckDB {
		constructor(logger: ConsoleLogger, worker: Worker)
		instantiate: (
			mainModule: string,
			pthreadWorker: string | null,
		) => Promise<void>
		connect: () => Promise<AsyncDuckDBConnection>
		terminate: () => Promise<void>
		registerFileBuffer: (name: string, buffer: Uint8Array) => Promise<void>
		registerFileURL: (
			name: string,
			url: string,
			protocol: DuckDBDataProtocol,
			allowOverwrite: boolean,
		) => Promise<void>
	}

	export interface DuckDBBundle {
		mainModule: string
		mainWorker: string | null
	}

	export interface DuckDBBundles {
		mvp: DuckDBBundle
		eh?: DuckDBBundle
	}

	export const getJsDelivrBundles: () => {
		mvp: DuckDBBundle
		eh: DuckDBBundle
	}
	export const selectBundle: (bundles: DuckDBBundles) => Promise<{
		mainWorker: string
		mainModule: string
		pthreadWorker: string | null
	}>
}
