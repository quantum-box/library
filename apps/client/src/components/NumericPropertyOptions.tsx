import { useEffect, useMemo, useState } from 'react'
import { useI18n } from '../i18n'
import { formatFormula, parseFormula } from '../lib/numericPropertyConfig'
import { fetchLibraryRepoProperties, fetchLibraryRepositories, type LibraryProperty } from '../lib/recordsApi'
import type { RepositoryPropertyDefinition } from '../lib/repositorySettingsApi'

interface Props {
  type: string
  initial?: string
  properties: RepositoryPropertyDefinition[]
  disabled: boolean
  onChange: (config: string | null) => void
}

export function NumericPropertyOptions({ type, initial, properties, disabled, onChange }: Props) {
  const { t } = useI18n()
  const saved = useMemo(() => { try { return JSON.parse(initial ?? '{}') } catch { return {} } }, [initial])
  const [precision, setPrecision] = useState(saved.decimal?.precision ?? saved.precision ?? 28)
  const [scale, setScale] = useState(saved.decimal?.scale ?? saved.scale ?? 12)
  const [unit, setUnit] = useState(saved.default_unit ?? 'g')
  const [expression, setExpression] = useState(() => saved.expression ? formatFormula(saved.expression, properties) : '0')
  const [relation, setRelation] = useState(saved.relation_property_id ?? '')
  const [targetId, setTargetId] = useState(saved.target_property_id ?? '')
  const [operation, setOperation] = useState(saved.function ?? 'sum')
  const [targets, setTargets] = useState<LibraryProperty[]>([])
  const [loadError, setLoadError] = useState(false)
  const [loading, setLoading] = useState(false)
  useEffect(() => {
    if (type !== 'ROLLUP' || !relation) return
    let cancelled = false
    const source = properties.find((p) => p.id === relation)
    setLoading(true); setTargets([]); setLoadError(false)
    void (async () => {
      const repositories = await fetchLibraryRepositories()
      const repository = repositories.find((r) => r.databases?.includes(source?.meta?.databaseId ?? ''))
      if (!repository) throw new Error('Missing repository')
      const result = await fetchLibraryRepoProperties({ org: repository.orgUsername ?? '', repo: repository.username, operatorId: repository.operatorId })
      if (!cancelled) setTargets(result.filter((p) => ['Decimal', 'Integer', 'Formula'].includes(p.typ)))
    })().catch(() => { if (!cancelled) setLoadError(true) }).finally(() => { if (!cancelled) setLoading(false) })
    return () => { cancelled = true }
  }, [relation, type, properties])

  const result = useMemo(() => {
    try {
      if (!Number.isInteger(precision) || !Number.isInteger(scale) || precision < 1 || precision > 28 || scale < 0 || scale > precision) return null
      const decimal = { precision, scale }
      if (type === 'DECIMAL') return JSON.stringify(decimal)
      if (type === 'QUANTITY') return JSON.stringify({ decimal, allowed_units: ['g', 'kg'].includes(unit) ? ['g', 'kg'] : ['mL', 'L'].includes(unit) ? ['mL', 'L'] : ['count'], default_unit: unit, unit_version: 1 })
      if (type === 'FORMULA') return JSON.stringify({ decimal, expression: parseFormula(expression, properties) })
      if (!relation || !targetId || loadError || loading) return null
      return JSON.stringify({ decimal, relation_property_id: relation, target_property_id: targetId, function: operation })
    } catch { return null }
  }, [precision, scale, type, unit, expression, properties, relation, targetId, operation, loadError, loading])
  useEffect(() => { onChange(result) }, [result, onChange])
  const inputClass = 'w-full rounded-md border border-input bg-background p-2 text-sm'
  return <div className="space-y-3">
    <div className="grid grid-cols-2 gap-3">
      <label className="text-sm">{t('numeric.precision')}<input className={inputClass} type="number" min={1} max={28} value={precision} disabled={disabled} onChange={(e) => setPrecision(Number(e.target.value))} /></label>
      <label className="text-sm">{t('numeric.scale')}<input className={inputClass} type="number" min={0} max={precision} value={scale} disabled={disabled} onChange={(e) => setScale(Number(e.target.value))} /></label>
    </div>
    {type === 'QUANTITY' && <label className="block text-sm">{t('numeric.unit')}<select className={inputClass} value={unit} disabled={disabled} onChange={(e) => setUnit(e.target.value)}>{['g', 'kg', 'mL', 'L', 'count'].map((u) => <option key={u}>{u}</option>)}</select></label>}
    {type === 'FORMULA' && <label className="block text-sm">{t('propertyType.formula')}<input className={inputClass} value={expression} disabled={disabled} onChange={(e) => setExpression(e.target.value)} /><span className="text-xs text-muted-foreground">{t('numeric.formulaHelp')} {properties.filter((p) => ['DECIMAL', 'INTEGER', 'FORMULA', 'ROLLUP'].includes(p.typ)).map((p) => p.name).join(', ')}</span></label>}
    {type === 'ROLLUP' && <>
      <label className="block text-sm">{t('propertyType.relation')}<select className={inputClass} value={relation} disabled={disabled} onChange={(e) => {setRelation(e.target.value); setTargetId('')}}><option value="">—</option>{properties.filter((p) => p.typ === 'RELATION').map((p) => <option key={p.id} value={p.id}>{p.displayName ?? p.name}</option>)}</select></label>
      <label className="block text-sm">{t('numeric.target')}<select className={inputClass} value={targetId} disabled={disabled || loading} onChange={(e) => setTargetId(e.target.value)}><option value="">—</option>{targets.map((p) => <option key={p.id} value={p.id}>{p.displayName ?? p.name}</option>)}</select></label>
      <label className="block text-sm">{t('numeric.operation')}<select className={inputClass} value={operation} disabled={disabled} onChange={(e) => setOperation(e.target.value)}>{['count','sum','avg','min','max'].map((op) => <option key={op}>{op}</option>)}</select></label>
    </>}
    {!result && <p role="alert" className="text-sm text-destructive">{t('numeric.invalid')}</p>}
  </div>
}
