import {
  buildSchema,
  type GraphQLField,
  type GraphQLObjectType,
} from 'graphql'
import { apiBaseUrl } from './config'

export interface SchemaField {
  name: string
  args: string[]
  type: string
  description?: string
}

export interface SchemaSummary {
  queries: SchemaField[]
  mutations: SchemaField[]
}

/**
 * Read the schema the API publishes as SDL and reduce it to the two lists a
 * reader needs: what can be asked, and what can be changed.
 *
 * Generating this from the live schema keeps the guide from describing a
 * field that has since been renamed or removed.
 */
export async function fetchSchemaSummary(
  signal?: AbortSignal,
): Promise<SchemaSummary> {
  const response = await fetch(`${apiBaseUrl}/v1/graphql/introspection`, {
    signal,
  })
  if (!response.ok) {
    throw new Error(
      `GraphQL スキーマの取得に失敗しました (HTTP ${response.status})`,
    )
  }
  return summarizeSdl(await response.text())
}

export function summarizeSdl(sdl: string): SchemaSummary {
  const schema = buildSchema(sdl)
  return {
    queries: describeFields(schema.getQueryType()),
    mutations: describeFields(schema.getMutationType()),
  }
}

function describeFields(
  type: GraphQLObjectType | null | undefined,
): SchemaField[] {
  if (!type) return []
  return Object.values(type.getFields())
    .map((field: GraphQLField<unknown, unknown>) => ({
      name: field.name,
      args: field.args.map(arg => `${arg.name}: ${arg.type.toString()}`),
      type: field.type.toString(),
      description: usableDescription(field.description),
    }))
    .sort((a, b) => a.name.localeCompare(b.name))
}

/**
 * Much of the schema still carries `TODO: add English documentation` as its
 * doc comment. Showing that to a reader is worse than showing nothing, so a
 * placeholder is treated as absent.
 */
function usableDescription(
  description: string | null | undefined,
): string | undefined {
  if (!description) return undefined
  const trimmed = description.trim()
  return trimmed.length > 0 && !trimmed.startsWith('TODO') ? trimmed : undefined
}
