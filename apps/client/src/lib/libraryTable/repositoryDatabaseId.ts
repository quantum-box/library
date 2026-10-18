/** Repo IDs (rp_) and relation target Database IDs (db_) are distinct. */
export function getRepositoryDatabaseId(repository: { id: string; databases?: string[] }): string | undefined {
  if (repository.databases?.[0]?.startsWith('db_')) return repository.databases[0]
  // Older integrations used a Database ID as their repository ID. Never use
  // a modern rp_ ID as a Relation target when database metadata is absent.
  return repository.id.startsWith('db_') ? repository.id : undefined
}
