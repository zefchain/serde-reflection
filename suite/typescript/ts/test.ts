import { test, suite } from "node:test"
import * as assert from "node:assert/strict"

import * as Registry from "./bincode/registry.ts"
import * as Data from "./data.ts"

suite("encode", (ctx) => {
	test("SimpleStruct", () => {
		const simple_instance = Registry.SimpleStruct.encode(Data.SimpleStruct_obj)
		assert.deepEqual(simple_instance, Data.SimpleStruct_bin)
	})
	test("MultiEnum_VariantC", () => {
		const enum_instance = Registry.MultiEnum.encode(Data.MultiEnum_VariantC_obj)
		assert.deepEqual(enum_instance, Data.MultiEnum_VariantC_bin)
	})
	test("MultiEnum unit variant", () => {
		const unit_variant = Registry.MultiEnum.encode(Data.MultiEnum_Unit_obj)
		assert.deepEqual(unit_variant, Data.MultiEnum_Unit_bin)
	})
	test("ComplexStruct", () => {
		const complex_instance = Registry.ComplexStruct.encode(Data.ComplexStruct_obj)
		assert.deepEqual(complex_instance, Data.ComplexStruct_bin)
	})

	test("ComplexStruct", () => {
		const complex_instance = Registry.ComplexStruct.encode(Data.ComplexStruct_obj)
		assert.deepEqual(complex_instance, Data.ComplexStruct_bin)
	})

	test("MultiEnum unit variant", () => {
		const bin = Registry.MultiEnum.encode({ $: "UnitVariant", UnitVariant: null })
		assert.deepEqual(bin, Data.MultiEnum_Unit_bin)
	})
})

suite("decode", () => {
	test("MultiEnum", () => {
		const enum_instance = Registry.MultiEnum.decode(Data.MultiEnum_VariantC_bin)
		assert.deepEqual(enum_instance, {
			$: "VariantC",
			VariantC: { x: 5, y: 3.14 }
		} satisfies Registry.MultiEnum)
	})

	test("MultiEnum unit variant", () => {
		const unit_variant = Registry.MultiEnum.decode(Data.MultiEnum_Unit_bin)
		assert.deepEqual(unit_variant, { $: "UnitVariant", UnitVariant: null } satisfies Registry.MultiEnum)
	})

	test("ComplexStruct", () => {
		const complex_instance = Registry.ComplexStruct.decode(Data.ComplexStruct_bin)
		assert.deepEqual(complex_instance, {
			inner: { a: 42, b: "Hello" },
			flag: true,
			items: [
				{ $: "VariantA", VariantA: 10 },
				{ $: "VariantB", VariantB: "World" }
			],
			unit: null,
			newtype: 99,
			tuple: [123, 45.67, "Test"]
		} satisfies Registry.ComplexStruct)
	})
})



