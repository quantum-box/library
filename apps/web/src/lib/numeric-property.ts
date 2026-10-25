type RepositoryPropertyDefinition = { id: string; name: string }

export type Expression = { op: string; [key: string]: unknown }

/** Parse the bounded formula language into the server's versioned AST. */
export function parseFormula(source: string, properties: Pick<RepositoryPropertyDefinition, 'id' | 'name'>[]): Expression {
  if (source.length > 4096) throw new Error('Formula is too long')
  const tokens = source.match(/(?:\[[A-Za-z][A-Za-z0-9_-]*\]|\d+(?:\.\d+)?|[A-Za-z_][A-Za-z_0-9]*|==|[()+\-*/,<])/g) ?? []
  if (tokens.join('') !== source.replace(/\s/g, '')) throw new Error('Invalid formula')
  let at = 0
  let depth = 0
  const take = (token: string) => { if (tokens[at++] !== token) throw new Error(`Expected ${token}`) }
  const primary = (): Expression => {
    if (++depth > 32) throw new Error('Formula is too complex')
    try {
      const token = tokens[at++]
      if (token === '-') return { op: 'subtract', left: { op: 'literal', value: '0' }, right: primary() }
      if (token === '(') { const value = comparison(); take(')'); return value }
      if (token && /^\d/.test(token)) return { op: 'literal', value: token }
      if (token === 'if') {
        take('('); const condition = comparison(); take(','); const then = comparison(); take(','); const otherwise = comparison(); take(')')
        return { op: 'if', condition, then, otherwise }
      }
      if (token === 'round') {
        take('('); const value = comparison(); take(','); const scale = tokens[at++]; take(')')
        if (!scale || !/^\d+$/.test(scale) || Number(scale) > 28) throw new Error('Invalid rounding scale')
        return { op: 'round', value, scale: Number(scale) }
      }
      const property = properties.find((p) => p.name === (token?.startsWith('[') ? token.slice(1, -1) : token))
      if (!property) throw new Error(`Unknown property: ${token ?? ''}`)
      return { op: 'property', id: property.id }
    } finally { depth-- }
  }
  const binary = (next: () => Expression, operators: Record<string, string>): Expression => {
    let left = next()
    while (operators[tokens[at]]) { const op = operators[tokens[at++]]; left = { op, left, right: next() } }
    return left
  }
  const product = () => binary(primary, { '*': 'multiply', '/': 'divide' })
  const sum = () => binary(product, { '+': 'add', '-': 'subtract' })
  const comparison = () => binary(sum, { '==': 'equal', '<': 'less_than' })
  const result = comparison()
  if (at !== tokens.length || tokens.length > 256) throw new Error('Invalid formula')
  return result
}

export function formatFormula(expression: Expression, properties: Pick<RepositoryPropertyDefinition, 'id' | 'name'>[]): string {
  const format = (value: unknown) => formatFormula(value as Expression, properties)
  if (expression.op === 'literal') return String(expression.value)
  if (expression.op === 'property') return `[${properties.find((p) => p.id === expression.id)?.name ?? String(expression.id)}]`
  if (expression.op === 'if') return `if(${format(expression.condition)}, ${format(expression.then)}, ${format(expression.otherwise)})`
  if (expression.op === 'round') return `round(${format(expression.value)}, ${expression.scale})`
  const signs: Record<string, string> = { add: '+', subtract: '-', multiply: '*', divide: '/', equal: '==', less_than: '<' }
  return `(${format(expression.left)} ${signs[expression.op]} ${format(expression.right)})`
}

/** Exact ordering for finite decimal strings, without converting to Number. */
export function compareDecimal(left: string, right: string): number {
  const parts = (value: string) => {
    if (!/^-?\d+(?:\.\d+)?$/.test(value)) return null
    const negative = value.startsWith('-')
    const [whole, fraction = ''] = (negative ? value.slice(1) : value).split('.')
    return { coefficient: BigInt(`${negative ? '-' : ''}${whole}${fraction}`), scale: fraction.length }
  }
  const a = parts(left), b = parts(right)
  if (!a || !b) return left.localeCompare(right)
  const scale = Math.max(a.scale, b.scale)
  const av = a.coefficient * 10n ** BigInt(scale - a.scale)
  const bv = b.coefficient * 10n ** BigInt(scale - b.scale)
  return av < bv ? -1 : av > bv ? 1 : 0
}

/** Compare quantities only after normalizing compatible units with integer arithmetic. */
export function compareQuantity(left: string, right: string): number {
  const parse = (text: string) => {
    const match = /^(-?\d+(?:\.\d+)?)\s+(g|kg|mL|L|count)$/.exec(text)
    if (!match) return null
    const [, amount, unit] = match
    const dimension = ['g', 'kg'].includes(unit) ? 'mass' : ['mL', 'L'].includes(unit) ? 'volume' : 'count'
    const [whole, fraction = ''] = amount.split('.')
    const factor = unit === 'kg' || unit === 'L' ? 1000n : 1n
    const coefficient = BigInt(`${whole}${fraction}`) * factor
    return { coefficient, scale: fraction.length, dimension }
  }
  const a = parse(left), b = parse(right)
  if (!a || !b || a.dimension !== b.dimension) return left.localeCompare(right)
  const scale = Math.max(a.scale, b.scale)
  const av = a.coefficient * 10n ** BigInt(scale - a.scale)
  const bv = b.coefficient * 10n ** BigInt(scale - b.scale)
  return av < bv ? -1 : av > bv ? 1 : 0
}
