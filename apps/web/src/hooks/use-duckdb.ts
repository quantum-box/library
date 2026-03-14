import type {
	DataFieldOnRepoPageFragment,
	PropertyFieldOnRepoPageFragment,
} from '@/gen/graphql'
import { PropertyType } from '@/gen/graphql'
import type {
	FilterConfig,
	SortConfig,
} from '@/app/v1beta/[org]/[repo]/data/components/data-toolbar'
import { useEffect, useMemo, useRef, useState } from 'react'
import type * as duckdb from '@duckdb/duckdb-wasm'
import { baseURL, platformId } from '@/lib/apiClient'

export type DuckDBStatus = 'idle' | 'loading' | 'ready' | 'error'
export type DuckDBSource = 'memory' | 'parquet'

interface UseDuckDBFilteredDataProps {
	org: string
	repo: string
	items: DataFieldOnRepoPageFragment[]
	properties: PropertyFieldOnRepoPageFragment[]
	filters: FilterConfig[]
	sortConfig: SortConfig | null
	searchQuery: string
	pagination: {
		currentPage: number
		itemsPerPage: number
		totalItems: number
	}
	sqlQuery: string
	isSqlMode: boolean
}

interface DuckDBQueryState {
	data: DataFieldOnRepoPageFragment[] | null
	status: DuckDBStatus
	queryTimeMs: number | null
	error: string | null
	source: DuckDBSource
	parquetError: string | null
}

interface ColumnDefinition {
	columnName: string
	propertyId?: string
	type: 'string' | 'number' | 'date'
}

const TABLE_NAME = 'data_view'

export function useDuckDBFilteredData({
	org,
	repo,
	items,
	properties,
	filters,
	sortConfig,
	searchQuery,
	pagination,
	sqlQuery,
	isSqlMode,
}: UseDuckDBFilteredDataProps): DuckDBQueryState {
	const dbRef = useRef<duckdb.AsyncDuckDB | null>(null)
	const connRef = useRef<duckdb.AsyncDuckDBConnection | null>(null)
	const [status, setStatus] = useState<DuckDBStatus>('idle')
	const [error, setError] = useState<string | null>(null)
	const [data, setData] = useState<DataFieldOnRepoPageFragment[] | null>(null)
	const [queryTimeMs, setQueryTimeMs] = useState<number | null>(null)
	const [dataReady, setDataReady] = useState(false)
	const [parquetReady, setParquetReady] = useState(false)
	const [parquetError, setParquetError] = useState<string | null>(null)

	const itemsKey = useMemo(
		() => items.map(item => `${item.id}:${item.updatedAt ?? ''}`).join('|'),
		[items],
	)
	const propertiesKey = useMemo(
		() => properties.map(prop => `${prop.id}:${prop.typ}`).join('|'),
		[properties],
	)

	const columnDefinitions = useMemo<ColumnDefinition[]>(() => {
		const baseColumns: ColumnDefinition[] = [
			{ columnName: 'id', type: 'string' },
			{ columnName: 'name', type: 'string' },
			{ columnName: 'created_at', type: 'date' },
			{ columnName: 'updated_at', type: 'date' },
		]

		const propertyColumns = properties.map<ColumnDefinition>(prop => {
			const type = propertyTypeToColumnType(prop.typ)
			return {
				columnName: `prop_${prop.id}`,
				propertyId: prop.id,
				type,
			}
		})

		return [...baseColumns, ...propertyColumns]
	}, [properties])

	const hasSqlQuery = useMemo(
		() => isSqlMode && sqlQuery.trim().length > 0,
		[isSqlMode, sqlQuery],
	)

	const shouldUseParquet = useMemo(
		() => pagination.totalItems > items.length,
		[pagination.totalItems, items.length],
	)
	const useParquet = shouldUseParquet && !parquetError
	const source: DuckDBSource = useParquet ? 'parquet' : 'memory'
	const tableReady = useParquet ? parquetReady : dataReady

	const shouldApplyPagination = useMemo(() => {
		const hasFilters = filters.length > 0
		const hasSearch = Boolean(searchQuery.trim())
		if (hasSqlQuery || useParquet) return false
		return pagination.totalItems === items.length && !hasFilters && !hasSearch
	}, [
		filters.length,
		hasSqlQuery,
		items.length,
		pagination.totalItems,
		searchQuery,
		useParquet,
	])

	const columnMap = useMemo(() => {
		const map = new Map<string, ColumnDefinition>()
		for (const column of columnDefinitions) {
			if (column.propertyId) {
				map.set(column.propertyId, column)
			}
		}
		return map
	}, [columnDefinitions])

	const itemsById = useMemo(() => {
		const map = new Map<string, DataFieldOnRepoPageFragment>()
		for (const item of items) {
			map.set(item.id, item)
		}
		return map
	}, [items])

	useEffect(() => {
		let cancelled = false

		const initialize = async () => {
			if (dbRef.current) return
			setStatus('loading')
			setError(null)

			try {
				const duckdb = await import('@duckdb/duckdb-wasm')
				const bundles = duckdb.getJsDelivrBundles()
				const bundle = await duckdb.selectBundle(bundles)
				if (!bundle.mainWorker) {
					throw new Error('DuckDB worker bundle is missing')
				}
				const workerUrl = URL.createObjectURL(
					new Blob([`importScripts("${bundle.mainWorker}");`], {
						type: 'text/javascript',
					}),
				)
				const worker = new Worker(workerUrl)
				const logger = new duckdb.ConsoleLogger('error')
				const db = new duckdb.AsyncDuckDB(logger, worker)
				await db.instantiate(bundle.mainModule, bundle.pthreadWorker)
				const conn = await db.connect()
				URL.revokeObjectURL(workerUrl)

				if (cancelled) {
					await conn.close()
					await db.terminate()
					return
				}

				dbRef.current = db
				connRef.current = conn
				setStatus('ready')
			} catch (initError) {
				if (cancelled) return
				setStatus('error')
				setError(normalizeError(initError))
			}
		}

		void initialize()

		return () => {
			cancelled = true
		}
	}, [])

	useEffect(() => {
		if (status !== 'ready') return
		if (!connRef.current) return
		if (useParquet) return

		let cancelled = false

		const loadData = async () => {
			const conn = connRef.current
			if (!conn) return
			setDataReady(false)
			setError(null)

			try {
				await conn.query(`DROP VIEW IF EXISTS ${quoteIdentifier(TABLE_NAME)}`)
				await conn.query(`DROP TABLE IF EXISTS ${quoteIdentifier(TABLE_NAME)}`)
				await conn.query(createTableSQL(columnDefinitions))

				if (items.length === 0) {
					setDataReady(true)
					return
				}

				const columnNames = columnDefinitions.map(column =>
					quoteIdentifier(column.columnName),
				)
				const insertColumns = columnNames.join(', ')
				const batches = chunkArray(items, 300)

				for (const batch of batches) {
					const values = batch
						.map(item => buildInsertValues(item, columnDefinitions, properties))
						.join(', ')
					if (!values) continue

					await conn.query(
						`INSERT INTO ${quoteIdentifier(TABLE_NAME)} (${insertColumns}) VALUES ${values}`,
					)
				}
				setDataReady(true)
			} catch (loadError) {
				if (cancelled) return
				setDataReady(false)
				setError(normalizeError(loadError))
			}
		}

		void loadData()

		return () => {
			cancelled = true
		}
	}, [
		status,
		itemsKey,
		propertiesKey,
		columnDefinitions,
		items,
		properties,
		useParquet,
	])

	useEffect(() => {
		if (!shouldUseParquet) {
			setParquetReady(false)
			setParquetError(null)
		}
	}, [shouldUseParquet])

	useEffect(() => {
		setParquetReady(false)
		setParquetError(null)
	}, [org, repo])

	useEffect(() => {
		if (!shouldUseParquet) return
		if (parquetError) return
		if (status !== 'ready') return
		if (!connRef.current) return

		let cancelled = false

		const loadParquet = async () => {
			const conn = connRef.current
			if (!conn) return
			setParquetReady(false)
			setParquetError(null)
			setError(null)

			try {
				const presignedUrl = await fetchParquetUrl({ org, repo })
				const fallbackName = `data-${org}-${repo}.parquet`
				const fileName = extractParquetFilename(presignedUrl, fallbackName)
				const buffer = await loadParquetBuffer({
					url: presignedUrl,
					fileName,
				})
				await dbRef.current?.registerFileBuffer(fileName, buffer)
				await conn.query(`DROP TABLE IF EXISTS ${quoteIdentifier(TABLE_NAME)}`)
				await conn.query(`DROP VIEW IF EXISTS ${quoteIdentifier(TABLE_NAME)}`)
				await conn.query(
					`CREATE OR REPLACE VIEW ${quoteIdentifier(TABLE_NAME)} AS SELECT * FROM '${fileName}'`,
				)
				if (cancelled) return
				setParquetReady(true)
			} catch (parquetLoadError) {
				if (cancelled) return
				setParquetError(normalizeError(parquetLoadError))
				setParquetReady(false)
			}
		}

		void loadParquet()

		return () => {
			cancelled = true
		}
	}, [org, repo, status, shouldUseParquet, parquetError])

	useEffect(() => {
		if (status !== 'ready') {
			setData(null)
			return
		}
		if (!connRef.current || !tableReady) {
			setData(null)
			return
		}

		let cancelled = false

		const runQuery = async () => {
			const conn = connRef.current
			if (!conn) return

			try {
				setError(null)
				setQueryTimeMs(null)
				const start = performance.now()
				const query = buildQuery({
					columnDefinitions,
					columnMap,
					filters,
					sortConfig,
					searchQuery,
					pagination,
					shouldApplyPagination,
					sqlQuery,
					isSqlMode,
					selectAll: useParquet,
				})
				const result = await conn.query(query)
				const mapped = useParquet
					? mapRowsToDataFields(result.toArray(), properties)
					: (result.toArray() as Array<{ id: string }>)
							.map(row => row.id)
							.map(id => itemsById.get(id))
							.filter((item): item is DataFieldOnRepoPageFragment =>
								Boolean(item),
							)

				if (cancelled) return
				setQueryTimeMs(Math.round(performance.now() - start))
				setData(mapped)
			} catch (queryError) {
				if (cancelled) return
				setError(normalizeError(queryError))
				setData(null)
				setQueryTimeMs(null)
			}
		}

		void runQuery()

		return () => {
			cancelled = true
		}
	}, [
		status,
		filters,
		sortConfig,
		searchQuery,
		sqlQuery,
		isSqlMode,
		columnDefinitions,
		columnMap,
		itemsById,
		pagination,
		shouldApplyPagination,
		tableReady,
		useParquet,
		properties,
	])

	return {
		data,
		status,
		queryTimeMs,
		error,
		source,
		parquetError,
	}
}

async function fetchParquetUrl({
	org,
	repo,
}: {
	org: string
	repo: string
}): Promise<string> {
	const headers: Record<string, string> = {
		'x-platform-id': platformId,
	}
	if (process.env.NODE_ENV === 'development') {
		headers.Authorization = 'Bearer dummy-token'
	}
	const response = await fetch(
		`${baseURL}/v1beta/repos/${org}/${repo}/data/parquet`,
		{
			headers,
		},
	)
	if (!response.ok) {
		throw new Error(`Failed to fetch parquet url (${response.status})`)
	}
	const payload = (await response.json()) as { presigned_url: string }
	return payload.presigned_url
}

function extractParquetFilename(url: string, fallback: string): string {
	try {
		const pathname = new URL(url).pathname
		const name = pathname.split('/').pop()
		return name || fallback
	} catch {
		return fallback
	}
}

async function loadParquetBuffer({
	url,
	fileName,
}: {
	url: string
	fileName: string
}): Promise<Uint8Array> {
	if (!('storage' in navigator) || !('getDirectory' in navigator.storage)) {
		const response = await fetch(url)
		const buffer = await response.arrayBuffer()
		return new Uint8Array(buffer)
	}

	const root = await navigator.storage.getDirectory()
	let fileHandle: FileSystemFileHandle | null = null

	try {
		fileHandle = await root.getFileHandle(fileName)
	} catch {
		fileHandle = await root.getFileHandle(fileName, { create: true })
		const response = await fetch(url)
		const blob = await response.blob()
		const writable = await fileHandle.createWritable()
		await writable.write(blob)
		await writable.close()
	}

	const file = await fileHandle.getFile()
	const buffer = await file.arrayBuffer()
	return new Uint8Array(buffer)
}

function mapRowsToDataFields(
	rows: Array<Record<string, unknown>>,
	properties: PropertyFieldOnRepoPageFragment[],
): DataFieldOnRepoPageFragment[] {
	return rows
		.map(row => mapRowToDataField(row, properties))
		.filter((row): row is DataFieldOnRepoPageFragment => Boolean(row))
}

function mapRowToDataField(
	row: Record<string, unknown>,
	properties: PropertyFieldOnRepoPageFragment[],
): DataFieldOnRepoPageFragment | null {
	const id = String(row.id ?? '')
	if (!id) return null
	const name = String(row.name ?? id)
	const createdAt = toIsoDateTime(row.created_at)
	const updatedAt = toIsoDateTime(row.updated_at)

	const propertyData: DataFieldOnRepoPageFragment['propertyData'] = properties
		.map(
			(
				property,
			): DataFieldOnRepoPageFragment['propertyData'][number] | null => {
				const columnName = `prop_${property.id}`
				const rawValue = row[columnName]
				const value = toPropertyValue(property.typ, rawValue)
				if (!value) return null
				return {
					__typename: 'PropertyData',
					propertyId: property.id,
					value,
				}
			},
		)
		.filter(
			(item): item is DataFieldOnRepoPageFragment['propertyData'][number] =>
				item !== null,
		)

	return {
		__typename: 'Data',
		id,
		name,
		createdAt,
		updatedAt,
		propertyData,
	}
}

function toIsoDateTime(value: unknown): string | null {
	if (!value) return null
	if (value instanceof Date) return value.toISOString()
	if (typeof value === 'number') return new Date(value).toISOString()
	if (typeof value === 'string') {
		const parsed = new Date(value)
		if (!Number.isNaN(parsed.getTime())) {
			return parsed.toISOString()
		}
	}
	return null
}

function toPropertyValue(
	propertyType: PropertyType,
	value: unknown,
): DataFieldOnRepoPageFragment['propertyData'][number]['value'] | null {
	if (value === null || value === undefined || value === '') return null
	const stringValue = String(value)

	switch (propertyType) {
		case PropertyType.Integer:
			return {
				__typename: 'IntegerValue',
				number: stringValue,
			}
		case PropertyType.Date:
			return {
				__typename: 'DateValue',
				date: toDateValue(value),
			}
		case PropertyType.Html:
			return { __typename: 'HtmlValue', html: stringValue }
		case PropertyType.Markdown:
			return { __typename: 'MarkdownValue' }
		case PropertyType.Relation: {
			const [databaseId, ...dataIds] = stringValue.split(',')
			if (!databaseId) return null
			return {
				__typename: 'RelationValue',
				databaseId,
				dataIds: dataIds.filter(Boolean),
			}
		}
		case PropertyType.Select:
			return { __typename: 'SelectValue', optionId: stringValue }
		case PropertyType.MultiSelect:
			return {
				__typename: 'MultiSelectValue',
				optionIds: stringValue.split(',').filter(Boolean),
			}
		case PropertyType.Location: {
			const [lat, lon] = stringValue.split(',')
			const latitude = Number(lat)
			const longitude = Number(lon)
			if (Number.isNaN(latitude) || Number.isNaN(longitude)) return null
			return {
				__typename: 'LocationValue',
				latitude,
				longitude,
			}
		}
		case PropertyType.Id:
			return { __typename: 'IdValue', id: stringValue }
		default:
			return { __typename: 'StringValue', string: stringValue }
	}
}

function toDateValue(value: unknown): string {
	if (value instanceof Date) return value.toISOString().split('T')[0]
	if (typeof value === 'number') {
		return new Date(value).toISOString().split('T')[0]
	}
	if (typeof value === 'string') {
		const parsed = new Date(value)
		if (!Number.isNaN(parsed.getTime())) {
			return parsed.toISOString().split('T')[0]
		}
	}
	return String(value)
}

function propertyTypeToColumnType(
	type: PropertyType,
): ColumnDefinition['type'] {
	switch (type) {
		case PropertyType.Integer:
			return 'number'
		case PropertyType.Date:
			return 'date'
		default:
			return 'string'
	}
}

function createTableSQL(columnDefinitions: ColumnDefinition[]): string {
	const columns = columnDefinitions
		.map(column =>
			[quoteIdentifier(column.columnName), duckdbType(column.type)].join(' '),
		)
		.join(', ')

	return `CREATE TABLE ${quoteIdentifier(TABLE_NAME)} (${columns})`
}

function duckdbType(type: ColumnDefinition['type']): string {
	switch (type) {
		case 'number':
			return 'DOUBLE'
		case 'date':
			return 'TIMESTAMP'
		default:
			return 'VARCHAR'
	}
}

function buildInsertValues(
	item: DataFieldOnRepoPageFragment,
	columnDefinitions: ColumnDefinition[],
	properties: PropertyFieldOnRepoPageFragment[],
): string {
	const propertyMap = new Map(properties.map(prop => [prop.id, prop.typ]))
	const values = columnDefinitions.map(column => {
		if (!column.propertyId) {
			return toSqlLiteral(getBaseField(item, column.columnName), column.type)
		}

		const propertyType = propertyMap.get(column.propertyId)
		const value = getPropertyValue(item, column.propertyId, propertyType)
		return toSqlLiteral(value, column.type)
	})

	return `(${values.join(', ')})`
}

function getBaseField(
	item: DataFieldOnRepoPageFragment,
	columnName: string,
): string | null {
	switch (columnName) {
		case 'id':
			return item.id
		case 'name':
			return item.name
		case 'created_at':
			return item.createdAt ?? null
		case 'updated_at':
			return item.updatedAt ?? null
		default:
			return null
	}
}

function getPropertyValue(
	item: DataFieldOnRepoPageFragment,
	propertyId: string,
	propertyType?: PropertyType,
): string | number | null {
	const propData = item.propertyData.find(pd => pd.propertyId === propertyId)
	if (!propData) return null
	const value = propData.value

	switch (value.__typename) {
		case 'StringValue':
			return value.string
		case 'IntegerValue': {
			const numberValue = Number(value.number)
			return Number.isFinite(numberValue) ? numberValue : null
		}
		case 'DateValue':
			return value.date
		case 'SelectValue':
			return value.optionId
		case 'MultiSelectValue':
			return value.optionIds.join(',')
		case 'HtmlValue':
			return value.html
		case 'LocationValue':
			return `${value.latitude},${value.longitude}`
		case 'IdValue':
			return value.id
		case 'RelationValue':
			return value.dataIds.join(',')
		case 'MarkdownValue':
			return null
		default:
			if (propertyType === PropertyType.Integer) return null
			return null
	}
}

function toSqlLiteral(
	value: string | number | null,
	type: ColumnDefinition['type'],
): string {
	if (value === null || value === undefined || value === '') {
		return 'NULL'
	}

	if (type === 'number') {
		return Number.isFinite(Number(value)) ? String(value) : 'NULL'
	}

	return `'${escapeSqlString(String(value))}'`
}

function escapeSqlString(value: string): string {
	return value.replace(/'/g, "''")
}

function quoteIdentifier(identifier: string): string {
	return `"${identifier.replace(/"/g, '""')}"`
}

function buildQuery({
	columnDefinitions,
	columnMap,
	filters,
	sortConfig,
	searchQuery,
	pagination,
	shouldApplyPagination,
	sqlQuery,
	isSqlMode,
	selectAll,
}: {
	columnDefinitions: ColumnDefinition[]
	columnMap: Map<string, ColumnDefinition>
	filters: FilterConfig[]
	sortConfig: SortConfig | null
	searchQuery: string
	pagination: {
		currentPage: number
		itemsPerPage: number
		totalItems: number
	}
	shouldApplyPagination: boolean
	sqlQuery: string
	isSqlMode: boolean
	selectAll: boolean
}): string {
	const trimmedSql = sqlQuery.trim()
	if (isSqlMode && trimmedSql) {
		const normalized = trimmedSql.toLowerCase()
		if (normalized.startsWith('select')) {
			return trimmedSql
		}
		return `SELECT id FROM ${quoteIdentifier(TABLE_NAME)} ${trimmedSql}`.trim()
	}

	const conditions: string[] = []
	const searchValue = searchQuery.trim()
	const searchColumns = columnDefinitions
		.filter(column => column.columnName !== 'id')
		.map(column => quoteIdentifier(column.columnName))

	if (searchValue) {
		const escapedSearch = escapeLike(searchValue.toLowerCase())
		const searchCondition = searchColumns
			.map(
				column =>
					`LOWER(CAST(${column} AS VARCHAR)) LIKE '%${escapedSearch}%' ESCAPE '\\'`,
			)
			.join(' OR ')
		conditions.push(`(${searchCondition})`)
	}

	for (const filter of filters) {
		const column =
			filter.propertyId === 'name'
				? { columnName: 'name' }
				: columnMap.get(filter.propertyId)
		if (!column) continue

		const columnExpr = `CAST(${quoteIdentifier(column.columnName)} AS VARCHAR)`
		const escapedValue = escapeLike(filter.value)

		switch (filter.operator) {
			case 'contains':
				conditions.push(
					`${columnExpr} IS NOT NULL AND ${columnExpr} LIKE '%${escapedValue}%' ESCAPE '\\'`,
				)
				break
			case 'equals':
				conditions.push(
					`${columnExpr} IS NOT NULL AND ${columnExpr} = '${escapeSqlString(filter.value)}'`,
				)
				break
			case 'notEquals':
				conditions.push(
					`${columnExpr} IS NOT NULL AND ${columnExpr} != '${escapeSqlString(filter.value)}'`,
				)
				break
			case 'isEmpty':
				conditions.push(`${columnExpr} IS NULL OR TRIM(${columnExpr}) = ''`)
				break
			case 'isNotEmpty':
				conditions.push(
					`${columnExpr} IS NOT NULL AND TRIM(${columnExpr}) != ''`,
				)
				break
			default:
				break
		}
	}

	const whereClause =
		conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''

	let orderClause = ''
	if (sortConfig) {
		const column =
			sortConfig.columnId === 'name'
				? 'name'
				: sortConfig.columnId === 'createdAt'
					? 'created_at'
					: sortConfig.columnId === 'updatedAt'
						? 'updated_at'
						: columnMap.get(sortConfig.columnId)?.columnName
		if (column) {
			orderClause = `ORDER BY ${quoteIdentifier(column)} ${sortConfig.direction.toUpperCase()}`
		}
	}

	const limitClause = shouldApplyPagination
		? `LIMIT ${pagination.itemsPerPage} OFFSET ${(pagination.currentPage - 1) * pagination.itemsPerPage}`
		: ''

	const selectClause = selectAll ? '*' : 'id'

	return `SELECT ${selectClause} FROM ${quoteIdentifier(TABLE_NAME)} ${whereClause} ${orderClause} ${limitClause}`.trim()
}

function escapeLike(value: string): string {
	return value.replace(/[\\%_]/g, match => `\\${match}`).replace(/'/g, "''")
}

function chunkArray<T>(items: T[], size: number): T[][] {
	const result: T[][] = []
	for (let i = 0; i < items.length; i += size) {
		result.push(items.slice(i, i + size))
	}
	return result
}

function normalizeError(error: unknown): string {
	if (error instanceof Error) return error.message
	return String(error)
}
