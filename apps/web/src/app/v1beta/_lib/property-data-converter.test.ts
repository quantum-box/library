import {
	PropertyDataForEditorFragment,
	PropertyForEditorFragment,
	PropertyType,
} from '@/gen/graphql'
import { describe, expect, it } from 'vitest'
import { convertPropertyData } from './property-data-converter'

describe('convertPropertyData', () => {
	it('round-trips a server-owned Id as its canonical value', () => {
		const properties = [
			{
				__typename: 'Property',
				id: 'prop_id',
				name: 'id',
				typ: PropertyType.Id,
				meta: { __typename: 'IdType', autoGenerate: true },
			},
		] as PropertyForEditorFragment[]
		const propertyData = [
			{
				__typename: 'PropertyData',
				propertyId: 'prop_id',
				value: {
					__typename: 'IdValue',
					id: 'data_01canonical',
				},
			},
		] as PropertyDataForEditorFragment[]

		expect(convertPropertyData(properties, propertyData)).toEqual([
			{
				propertyId: 'prop_id',
				value: { string: 'data_01canonical' },
			},
		])
	})

	it('omits a stale empty value for a server-owned Id', () => {
		const properties = [
			{
				__typename: 'Property',
				id: 'prop_id',
				name: 'id',
				typ: PropertyType.Id,
				meta: { __typename: 'IdType', autoGenerate: true },
			},
		] as PropertyForEditorFragment[]
		const propertyData = [
			{
				__typename: 'PropertyData',
				propertyId: 'prop_id',
				value: { __typename: 'StringValue', string: '' },
			},
		] as PropertyDataForEditorFragment[]

		expect(convertPropertyData(properties, propertyData)).toEqual([])
	})

	it('keeps a manually managed Id editable', () => {
		const properties = [
			{
				__typename: 'Property',
				id: 'prop_external_id',
				name: 'external_id',
				typ: PropertyType.Id,
				meta: { __typename: 'IdType', autoGenerate: false },
			},
		] as PropertyForEditorFragment[]
		const propertyData = [
			{
				__typename: 'PropertyData',
				propertyId: 'prop_external_id',
				value: { __typename: 'StringValue', string: 'external-123' },
			},
		] as PropertyDataForEditorFragment[]

		expect(convertPropertyData(properties, propertyData)).toEqual([
			{
				propertyId: 'prop_external_id',
				value: { string: 'external-123' },
			},
		])
	})
})
