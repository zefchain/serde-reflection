import type * as $t from "./serde.ts"
import { BincodeReader, BincodeWriter } from "./bincode.ts"

export type ComplexStruct = {
	inner: SimpleStruct,
	flag: $t.bool,
	items: $t.Seq<MultiEnum>,
	unit: UnitStruct,
	newtype: NewtypeStruct,
	tuple: TupleStruct,
	map: $t.Map<$t.i32, $t.i64>,
}

export type MultiEnum = 
	| { $: "VariantA", VariantA: $t.i32 }
	| { $: "VariantB", VariantB: $t.str }
	| { $: "VariantC", VariantC: { x: $t.u8, y: $t.f64 } }
	| { $: "UnitVariant", UnitVariant: $t.unit }

export type NewtypeStruct = $t.i32

export type SimpleStruct = {
	a: $t.u32,
	b: $t.str,
}

export type TupleStruct = [$t.i32, $t.f64, $t.str]

export type UnitStruct = $t.unit

export const ComplexStruct = {
	encode(value: ComplexStruct, writer = new BincodeWriter()) {
		SimpleStruct.encode(value.inner, writer)
		writer.writeBool(value.flag)
		writer.writeLength(value.items.length)
		for (const item of value.items) {
			MultiEnum.encode(item, writer)
		}
		UnitStruct.encode(value.unit, writer)
		NewtypeStruct.encode(value.newtype, writer)
		TupleStruct.encode(value.tuple, writer)
		writer.writeMap(value.map, writer.writeI32.bind(writer), writer.writeI64.bind(writer))
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value = {} as ComplexStruct
		value.inner = SimpleStruct.decode(input, reader)
		value.flag = reader.readBool()
		value.items = reader.readList<MultiEnum>(() => MultiEnum.decode(input, reader))
		value.unit = UnitStruct.decode(input, reader)
		value.newtype = NewtypeStruct.decode(input, reader)
		value.tuple = TupleStruct.decode(input, reader)
		value.map = reader.readMap<$t.i32, $t.i64>(reader.readI32.bind(reader), reader.readI64.bind(reader))
		return value
	}
}

export const MultiEnum = {
	encode(value: MultiEnum, writer = new BincodeWriter()) {
		switch (value.$) {
			case "VariantA": {
				writer.writeVariantIndex(0)
				writer.writeI32(value.VariantA)
				break
			}
			case "VariantB": {
				writer.writeVariantIndex(1)
				writer.writeString(value.VariantB)
				break
			}
			case "VariantC": {
				writer.writeVariantIndex(2)
				writer.writeU8(value.VariantC.x)
				writer.writeF64(value.VariantC.y)
				break
			}
			case "UnitVariant": {
				writer.writeVariantIndex(3)
				writer.writeUnit(value.UnitVariant)
				break
			}
		}
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		let value: MultiEnum
		switch (reader.readVariantIndex()) {
			case 0: {
				value = { $: "VariantA" } as $t.WrapperOfCase<MultiEnum, "VariantA">
				value.VariantA = reader.readI32()
				break
			}
			case 1: {
				value = { $: "VariantB" } as $t.WrapperOfCase<MultiEnum, "VariantB">
				value.VariantB = reader.readString()
				break
			}
			case 2: {
				value = { $: "VariantC" } as $t.WrapperOfCase<MultiEnum, "VariantC">
				value.VariantC = {} as $t.WrapperOfCase<MultiEnum, "VariantC">["VariantC"]
				value.VariantC.x = reader.readU8()
				value.VariantC.y = reader.readF64()
				break
			}
			case 3: {
				value = { $: "UnitVariant" } as $t.WrapperOfCase<MultiEnum, "UnitVariant">
				value.UnitVariant = reader.readUnit()
				break
			}
		}

		return value
	}
}

export const NewtypeStruct = {
	encode(value: NewtypeStruct, writer = new BincodeWriter()) {
		writer.writeI32(value)
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: NewtypeStruct = reader.readI32()
		return value
	}
}

export const SimpleStruct = {
	encode(value: SimpleStruct, writer = new BincodeWriter()) {
		writer.writeU32(value.a)
		writer.writeString(value.b)
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value = {} as SimpleStruct
		value.a = reader.readU32()
		value.b = reader.readString()
		return value
	}
}

export const TupleStruct = {
	encode(value: TupleStruct, writer = new BincodeWriter()) {
		writer.writeI32(value[0])
		writer.writeF64(value[1])
		writer.writeString(value[2])
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: TupleStruct = [reader.readI32(), reader.readF64(), reader.readString()]
		return value
	}
}

export const UnitStruct = {
	encode(value: UnitStruct, writer = new BincodeWriter()) {
		writer.writeUnit(null)
		return writer.getBytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: $t.unit = reader.readUnit()
		return value
	}
}
