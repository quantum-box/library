import {
  fetchLibraryDataDetail,
  fetchLibraryRepositories,
  fetchLibraryRepoTableData,
  type LibraryRepository,
} from '../recordsApi'

export interface RelationRecordOption {
  id: string
  name: string
  unavailable?: boolean
}

export interface RelationRecordPage {
  items: RelationRecordOption[]
  hasMore: boolean
  nextPage?: number
  repositoryLabel: string
}

export interface LibraryRelationRecordLoader {
  loadPage: (databaseId: string, page?: number) => Promise<RelationRecordPage>
  loadSelected: (databaseId: string, dataIds: readonly string[]) => Promise<RelationRecordOption[]>
}

interface RelationTarget {
  org: string
  repo: string
  operatorId?: string
  databaseId: string
  repoName: string
}

function relationTarget(
  repositories: readonly LibraryRepository[],
  databaseId: string,
): RelationTarget {
  const repository = repositories.find((candidate) => candidate.id === databaseId)
  if (!repository?.orgUsername) {
    throw new Error(`Relation target repository ${databaseId} is unavailable`)
  }
  return {
    org: repository.orgUsername,
    repo: repository.username,
    operatorId: repository.operatorId,
    databaseId: repository.id,
    repoName: repository.name || repository.username,
  }
}

/**
 * Creates one loader per repository screen.
 *
 * Repository discovery and resolved labels are shared by every Relation cell
 * on that screen, while a new screen gets a fresh cache after an auth or route
 * change. The API remains paged: opening one picker never downloads the whole
 * target repository.
 */
export function createLibraryRelationRecordLoader(): LibraryRelationRecordLoader {
  let repositoriesPromise: Promise<LibraryRepository[]> | null = null
  const recordCache = new Map<string, RelationRecordOption>()

  const repositories = () => {
    repositoriesPromise ??= fetchLibraryRepositories().catch((error: unknown) => {
      // A failed discovery must not poison the loader forever: the picker has
      // an explicit retry action and expects the next attempt to reach the API.
      repositoriesPromise = null
      throw error
    })
    return repositoriesPromise
  }

  const target = async (databaseId: string) =>
    relationTarget(await repositories(), databaseId)

  const remember = (databaseId: string, items: readonly RelationRecordOption[]) => {
    for (const item of items) recordCache.set(`${databaseId}:${item.id}`, item)
  }

  return {
    async loadPage(databaseId, page = 1) {
      const resolved = await target(databaseId)
      const result = await fetchLibraryRepoTableData(resolved, page)
      const items = result.items.map((item) => ({ id: item.id, name: item.name }))
      remember(databaseId, items)
      return {
        items,
        hasMore: result.hasMore,
        nextPage: result.nextPage,
        repositoryLabel: `${resolved.org} / ${resolved.repoName}`,
      }
    },

    async loadSelected(databaseId, dataIds) {
      const resolved = await target(databaseId)
      const uniqueIds = [...new Set(dataIds)]
      const loaded = await Promise.all(uniqueIds.map(async (dataId) => {
        const cached = recordCache.get(`${databaseId}:${dataId}`)
        if (cached) return cached
        try {
          const detail = await fetchLibraryDataDetail(dataId, resolved)
          const item = { id: detail.item.id, name: detail.item.name }
          remember(databaseId, [item])
          return item
        } catch {
          // A selected record may have been removed or become unreadable. Keep
          // its id visible and selected so merely opening the editor is never
          // a destructive write.
          return { id: dataId, name: dataId, unavailable: true }
        }
      }))
      return loaded
    },
  }
}
