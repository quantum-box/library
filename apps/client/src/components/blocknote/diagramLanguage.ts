export function diagramLanguage(language: string) {
  const normalized = language.trim().toLowerCase()
  return normalized === 'mermaid' || normalized === 'mmd' ? 'mermaid'
    : normalized === 'plantuml' || normalized === 'puml' ? 'plantuml' : undefined
}

