import { describe, expect, it } from 'vitest'

import { normalizeValueForInput } from './actions'

describe('normalizeValueForInput', () => {
  it('preserves a typed Id value during an inline record update', () => {
    expect(
      normalizeValueForInput({
        __typename: 'IdValue',
        id: 'data_01j00000000000000000000000',
      }),
    ).toEqual({
      string: 'data_01j00000000000000000000000',
    })
  })
})
