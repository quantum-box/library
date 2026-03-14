import {
	DataForDataDetailFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import yaml from 'js-yaml'

const isMarkdownProperty = (property: PropertyForEditorFragment) =>
	property.typ === PropertyType.Markdown

const isHtmlProperty = (property: PropertyForEditorFragment) =>
	property.typ === PropertyType.Html

const isBodyProperty = (property: PropertyForEditorFragment) =>
	isMarkdownProperty(property) || isHtmlProperty(property)

const CONTENT_KEY = 'content'

/**
 * GitHub sync extension configuration
 */
export interface ExtGithub {
	/** GitHub repository in "owner/repo" format (required) */
	repo: string
	/** Target path in the repository (required) */
	path: string
}

/**
 * Extended frontmatter options for GitHub sync
 */
export interface FrontmatterOptions {
	/** Tags associated with the content */
	tags?: string[]
	/** Summary/description of the content */
	summary?: string
	/** Last updated timestamp (ISO 8601) */
	updatedAt?: string
	/** Source URL or reference */
	source?: string
	/** Target directory for export (overrides default) */
	directory?: string
	/** Category for directory resolution */
	category?: string
	/** GitHub sync extension configuration */
	ext_github?: ExtGithub
	/** Additional extension fields (must start with ext_) */
	[key: `ext_${string}`]: unknown
}

/** Allowed extension prefixes */
const ALLOWED_EXT_KEYS = ['ext_github'] as const
type AllowedExtKey = (typeof ALLOWED_EXT_KEYS)[number]

/**
 * Validates that all ext_ prefixed fields are allowed
 */
export const validateExtensionFields = (
	options: FrontmatterOptions,
): { valid: boolean; invalidKeys: string[] } => {
	const invalidKeys: string[] = []
	for (const key of Object.keys(options)) {
		if (
			key.startsWith('ext_') &&
			!ALLOWED_EXT_KEYS.includes(key as AllowedExtKey)
		) {
			invalidKeys.push(key)
		}
	}
	return { valid: invalidKeys.length === 0, invalidKeys }
}

/**
 * Required frontmatter fields for GitHub sync
 */
export const REQUIRED_FRONTMATTER_FIELDS = [
	'id',
	'title',
	'summary',
	'tags',
	'updatedAt',
] as const

/**
 * Validates that all required frontmatter fields are present
 */
export const validateRequiredFields = (
	frontmatter: Record<string, unknown>,
): { valid: boolean; missingFields: string[] } => {
	const missingFields: string[] = []
	for (const field of REQUIRED_FRONTMATTER_FIELDS) {
		const value = frontmatter[field]
		if (value === undefined || value === null || value === '') {
			missingFields.push(field)
		}
	}
	return { valid: missingFields.length === 0, missingFields }
}

export const buildFrontmatter = (
	properties: PropertyForEditorFragment[],
	data?: DataForDataDetailFragment | null,
	options?: FrontmatterOptions,
) => {
	if (!data) return {}

	const propertyMap = new Map(properties.map(p => [p.id, p]))
	const frontmatter: Record<string, unknown> = {
		title: data.name,
		id: data.id,
	}

	// Add optional fields from options
	if (options?.summary) frontmatter.summary = options.summary
	if (options?.tags && options.tags.length > 0) frontmatter.tags = options.tags
	if (options?.updatedAt) frontmatter.updatedAt = options.updatedAt
	if (options?.source) frontmatter.source = options.source
	if (options?.directory) frontmatter.directory = options.directory
	if (options?.category) frontmatter.category = options.category

	// Add ext_github extension if provided
	if (options?.ext_github) {
		frontmatter.ext_github = {
			repo: options.ext_github.repo,
			path: options.ext_github.path,
		}
	}

	for (const propertyData of data.propertyData) {
		const property = propertyMap.get(propertyData.propertyId)
		if (!property || isBodyProperty(property)) continue

		const key = property.name
		const value = propertyData.value

		switch (value.__typename) {
			case 'StringValue':
				frontmatter[key] = value.string
				break
			case 'IntegerValue':
				frontmatter[key] = Number.isNaN(Number(value.number))
					? value.number
					: Number(value.number)
				break
			case 'SelectValue':
				frontmatter[key] = value.optionId
				break
			case 'MultiSelectValue':
				frontmatter[key] = value.optionIds
				break
			case 'RelationValue':
				frontmatter[key] = {
					databaseId: value.databaseId,
					dataIds: value.dataIds,
				}
				break
			default:
				break
		}
	}

	return frontmatter
}

export const pickBody = (
	properties: PropertyForEditorFragment[],
	data?: DataForDataDetailFragment | null,
) => {
	if (!data) return ''

	const propertyMap = new Map(properties.map(p => [p.id, p]))

	// 明示的に content プロパティを優先
	for (const propertyData of data.propertyData) {
		const property = propertyMap.get(propertyData.propertyId)
		if (!property) continue
		if (property.name?.toLowerCase() === CONTENT_KEY) {
			if (
				propertyData.value.__typename === 'MarkdownValue' &&
				propertyData.value.markdown
			) {
				return propertyData.value.markdown
			}
			if (
				propertyData.value.__typename === 'HtmlValue' &&
				propertyData.value.html
			) {
				return propertyData.value.html
			}
			if (propertyData.value.__typename === 'StringValue') {
				return propertyData.value.string ?? ''
			}
		}
	}

	for (const propertyData of data.propertyData) {
		const property = propertyMap.get(propertyData.propertyId)
		if (!property) continue

		if (
			isMarkdownProperty(property) &&
			propertyData.value.__typename === 'MarkdownValue'
		) {
			return propertyData.value.markdown ?? ''
		}
	}

	for (const propertyData of data.propertyData) {
		const property = propertyMap.get(propertyData.propertyId)
		if (!property) continue

		if (
			isHtmlProperty(property) &&
			propertyData.value.__typename === 'HtmlValue'
		) {
			return propertyData.value.html ?? ''
		}
	}

	return `# ${data.name}\n\n` // fallback
}

export interface ComposeMarkdownOptions extends FrontmatterOptions {
	/** Skip required field validation (useful for preview) */
	skipValidation?: boolean
}

export interface ComposeMarkdownResult {
	markdown: string
	frontmatter: Record<string, unknown>
	validation: {
		valid: boolean
		missingFields: string[]
		invalidExtKeys: string[]
	}
}

/**
 * Composes a complete Markdown document with frontmatter
 */
export const composeMarkdown = (
	properties: PropertyForEditorFragment[],
	data?: DataForDataDetailFragment | null,
	options?: ComposeMarkdownOptions,
): ComposeMarkdownResult => {
	const frontmatter = buildFrontmatter(properties, data, options)
	const body = pickBody(properties, data)

	// Validate extension fields
	const extValidation = options
		? validateExtensionFields(options)
		: { valid: true, invalidKeys: [] }

	// Validate required fields
	const reqValidation = validateRequiredFields(frontmatter)

	// Sort keys for deterministic output (idempotent)
	const sortedFrontmatter = sortFrontmatterKeys(frontmatter)
	const frontmatterYaml = yaml.dump(sortedFrontmatter, {
		skipInvalid: true,
		sortKeys: false, // We already sorted manually
		lineWidth: -1, // Disable line wrapping for consistent output
		noRefs: true, // Disable YAML references
	})

	return {
		markdown: `---\n${frontmatterYaml}---\n\n${body}\n`,
		frontmatter: sortedFrontmatter,
		validation: {
			valid: reqValidation.valid && extValidation.valid,
			missingFields: reqValidation.missingFields,
			invalidExtKeys: extValidation.invalidKeys,
		},
	}
}

/**
 * Sort frontmatter keys in a deterministic order for idempotent output
 * Priority: title, id, summary, tags, updatedAt, source, directory, category,
 * then alphabetically for custom properties, then ext_ fields at the end
 */
const PRIORITY_KEYS = [
	'title',
	'id',
	'summary',
	'tags',
	'updatedAt',
	'source',
	'directory',
	'category',
] as const

const sortFrontmatterKeys = (
	frontmatter: Record<string, unknown>,
): Record<string, unknown> => {
	const result: Record<string, unknown> = {}
	const keys = Object.keys(frontmatter)

	// Add priority keys first (in order)
	for (const key of PRIORITY_KEYS) {
		if (keys.includes(key) && frontmatter[key] !== undefined) {
			result[key] = frontmatter[key]
		}
	}

	// Add remaining non-ext keys (alphabetically)
	const remainingKeys = keys
		.filter(k => !PRIORITY_KEYS.includes(k as (typeof PRIORITY_KEYS)[number]))
		.filter(k => !k.startsWith('ext_'))
		.sort()
	for (const key of remainingKeys) {
		result[key] = frontmatter[key]
	}

	// Add ext_ keys last (alphabetically)
	const extKeys = keys.filter(k => k.startsWith('ext_')).sort()
	for (const key of extKeys) {
		result[key] = frontmatter[key]
	}

	return result
}

/**
 * Legacy compose function for backward compatibility
 * @deprecated Use composeMarkdown with options instead
 */
export const composeMarkdownSimple = (
	properties: PropertyForEditorFragment[],
	data?: DataForDataDetailFragment | null,
): string => {
	const result = composeMarkdown(properties, data)
	return result.markdown
}
