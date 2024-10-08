import type * as $t from "./serde"
import { BincodeReader, BincodeWriter } from "./bincode"

export type ComplexStruct = {
	inner: SimpleStruct,
	flag: $t.bool,
	items: $t.Seq<MultiEnum>,
	unit: UnitStruct,
	newtype: NewtypeStruct,
	tuple: TupleStruct,
	tupple_inline: $t.Tuple<[$t.str, $t.i32]>,
	map: $t.Map<$t.i32, $t.i64>,
}

export type MultiEnum = 
	| { $: "variant_a", $0: $t.i32 }
	| { $: "variant_b", $0: $t.str }
	| { $: "variant_c", x: $t.u8, y: $t.f64 }
	| { $: "unit_variant", $0?: $t.unit }

export type NewtypeStruct = $t.i32

export type SimpleStruct = {
	a: $t.u32,
	b: $t.str,
}

export type TupleStruct = $t.Tuple<[$t.i32, $t.f64, $t.str]>

export type UnitStruct = $t.unit

export const ComplexStruct = {
	encode(value: ComplexStruct, writer = new BincodeWriter()) {
		SimpleStruct.encode(value.inner, writer)
		writer.write_bool(value.flag)
		writer.write_length(value.items.length)
		for (const item of value.items) {
			MultiEnum.encode(item, writer)
		}
		UnitStruct.encode(value.unit, writer)
		NewtypeStruct.encode(value.newtype, writer)
		TupleStruct.encode(value.tuple, writer)
		writer.write_string(value.tupple_inline.$0)
		writer.write_i32(value.tupple_inline.$1)
		writer.write_map(value.map, writer.write_i32.bind(writer), writer.write_i64.bind(writer))
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: ComplexStruct = {
			inner: SimpleStruct.decode(input, reader),
			flag: reader.read_bool(),
			items: reader.read_list<MultiEnum>(() => MultiEnum.decode(input, reader)),
			unit: UnitStruct.decode(input, reader),
			newtype: NewtypeStruct.decode(input, reader),
			tuple: TupleStruct.decode(input, reader),
			tupple_inline: {
				$0: reader.read_string(),
				$1: reader.read_i32(),
			},
			map: reader.read_map<$t.i32, $t.i64>(reader.read_i32.bind(reader), reader.read_i64.bind(reader)),
		}
		return value
	}
}

export const MultiEnum = {
	encode(value: MultiEnum, writer = new BincodeWriter()) {
		switch (value.$) {
			case "variant_a": {
				writer.write_variant_index(0)
				writer.write_i32(value.$0)
				break
			}
			case "variant_b": {
				writer.write_variant_index(1)
				writer.write_string(value.$0)
				break
			}
			case "variant_c": {
				writer.write_variant_index(2)
				writer.write_u8(value.x)
				writer.write_f64(value.y)
				break
			}
			case "unit_variant": {
				writer.write_variant_index(3)
				writer.write_unit(value.$0)
				break
			}
		}
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		let value: MultiEnum
		switch (reader.read_variant_index()) {
			case 0: {
				value = {
					$: "variant_a",
					$0: reader.read_i32(),
				} satisfies Extract<MultiEnum, { $: "variant_a" }>
				break
			}
			case 1: {
				value = {
					$: "variant_b",
					$0: reader.read_string(),
				} satisfies Extract<MultiEnum, { $: "variant_b" }>
				break
			}
			case 2: {
				value = {
					$: "variant_c",
					x: reader.read_u8(),
					y: reader.read_f64(),
				} satisfies Extract<MultiEnum, { $: "variant_c" }>
				break
			}
			case 3: {
				value = {
					$: "unit_variant",
					$0: reader.read_unit()
				} satisfies Extract<MultiEnum, { $: "unit_variant" }>
				break
			}
		}

		return value
	}
}

export const NewtypeStruct = {
	encode(value: NewtypeStruct, writer = new BincodeWriter()) {
		writer.write_i32(value)
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: NewtypeStruct = reader.read_i32()
		return value
	}
}

export const SimpleStruct = {
	encode(value: SimpleStruct, writer = new BincodeWriter()) {
		writer.write_u32(value.a)
		writer.write_string(value.b)
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: SimpleStruct = {
			a: reader.read_u32(),
			b: reader.read_string(),
		}
		return value
	}
}

export const TupleStruct = {
	encode(value: TupleStruct, writer = new BincodeWriter()) {
		writer.write_i32(value.$0)
		writer.write_f64(value.$1)
		writer.write_string(value.$2)
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: TupleStruct = {
			$0: reader.read_i32(),
			$1: reader.read_f64(),
			$2: reader.read_string(),
		}
		return value
	}
}

export const UnitStruct = {
	encode(value: UnitStruct, writer = new BincodeWriter()) {
		writer.write_unit(null)
		return writer.get_bytes()
	},
	decode(input: Uint8Array, reader = new BincodeReader(input)) {
		const value: $t.unit = reader.read_unit()
		return value
	}
}
