import { describe, expect, it } from 'vitest'
import { compareDecimal, compareQuantity, formatFormula, parseFormula } from './numericPropertyConfig'
import { libraryPropertyValueToGraphqlInput, parseEditablePropertyValue } from './libraryTable/libraryPropertyInput'
import { propertyValueText } from './libraryTable/libraryPropertyFormat'

describe('numeric property contracts', () => {
  const properties = [{ id: 'cost-id', name: 'cost' }, { id: 'servings-id', name: 'servings' }]
  it('compiles a bounded formula with stable references and precedence', () => {
    const ast = parseFormula('round(cost / servings, 2)', properties)
    expect(ast).toEqual({ op: 'round', value: { op: 'divide', left: { op: 'property', id: 'cost-id' }, right: { op: 'property', id: 'servings-id' } }, scale: 2 })
    expect(parseFormula(formatFormula(ast, properties), properties)).toEqual(ast)
    expect(() => parseFormula('fetch("http://example.com")', properties)).toThrow()
    expect(() => parseFormula('missing + 1', properties)).toThrow()
    expect(() => parseFormula('('.repeat(40) + '1' + ')'.repeat(40), properties)).toThrow()
  })
  it('sorts arbitrary decimal precision without rounding', () => {
    expect(compareDecimal('2', '10')).toBe(-1)
    expect(compareDecimal('-2', '-10')).toBe(1)
    expect(compareDecimal('1.000000000000000001', '1.000000000000000002')).toBe(-1)
    expect(compareDecimal('-0.00', '0')).toBe(0)
  })
  it('normalizes compatible units before ordering', () => {
    expect(compareQuantity('1.5 kg', '150 g')).toBe(1)
    expect(compareQuantity('1 L', '1000 mL')).toBe(0)
    expect(compareQuantity('-1 kg', '-900 g')).toBe(-1)
  })
  it('round-trips quantity input and keeps computed values read-only', () => {
    const quantity = { id: 'q', name: 'mass', typ: 'Quantity' }
    const value = parseEditablePropertyValue(quantity, '1.5 kg')!
    expect(value).toEqual({ quantity: { value: '1.5', unit: 'kg' } })
    expect(propertyValueText(quantity, value)).toBe('1.5 kg')
    expect(libraryPropertyValueToGraphqlInput(quantity, value)).toEqual({ quantity: '{"value":"1.5","unit":"kg"}' })
    expect(libraryPropertyValueToGraphqlInput({ ...quantity, typ: 'Formula' }, { formula: { status: 'ready', value: '125' } })).toBeNull()
    expect(propertyValueText({ ...quantity, typ: 'Formula' }, { formula: '{"status":"ready","value":"125"}' })).toBe('125')
    expect(parseEditablePropertyValue({ ...quantity, typ: 'Decimal' }, '0.100000000001')).toEqual({ decimal: '0.100000000001' })
  })
})
