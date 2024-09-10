
import { Serializer, Deserializer } from '../serde/mod.ts';
import { Seq, bool, int32, int64, uint32, float32, float64, str } from '../serde/mod.ts';

export abstract class Enum {
	abstract serialize(serializer: Serializer): void;

	static deserialize(deserializer: Deserializer): Enum {
		const index = deserializer.deserializeVariantIndex();
		switch (index) {
			case 0: return EnumVariantONE.load(deserializer);
			case 1: return EnumVariantTWO.load(deserializer);
			case 2: return EnumVariantTHREE.load(deserializer);
			case 3: return EnumVariantFOUR.load(deserializer);
			case 4: return EnumVariantFIVE.load(deserializer);
			default: throw new Error("Unknown variant index for Enum: " + index);
		}
	}
}


export class EnumVariantONE extends Enum {
	constructor() {
		super();
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeVariantIndex(0);
	}

	static load(deserializer: Deserializer): EnumVariantONE {
		return new EnumVariantONE();
	}

}

export class EnumVariantTWO extends Enum {
	constructor() {
		super();
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeVariantIndex(1);
	}

	static load(deserializer: Deserializer): EnumVariantTWO {
		return new EnumVariantTWO();
	}

}

export class EnumVariantTHREE extends Enum {
	constructor() {
		super();
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeVariantIndex(2);
	}

	static load(deserializer: Deserializer): EnumVariantTHREE {
		return new EnumVariantTHREE();
	}

}

export class EnumVariantFOUR extends Enum {
	constructor() {
		super();
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeVariantIndex(3);
	}

	static load(deserializer: Deserializer): EnumVariantFOUR {
		return new EnumVariantFOUR();
	}

}

export class EnumVariantFIVE extends Enum {
	constructor() {
		super();
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeVariantIndex(4);
	}

	static load(deserializer: Deserializer): EnumVariantFIVE {
		return new EnumVariantFIVE();
	}

}
export class Inner {

	constructor(public int32: int32, public inner_inner: InnerInner, public outer: Outer) {
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeI32(this.int32);
		this.inner_inner.serialize(serializer);
		this.outer.serialize(serializer);
	}

	static deserialize(deserializer: Deserializer): Inner {
		const int32 = deserializer.deserializeI32();
		const inner_inner = InnerInner.deserialize(deserializer);
		const outer = Outer.deserialize(deserializer);
		return new Inner(int32, inner_inner, outer);
	}

}
export class InnerInner {

	constructor(public long: int64, public enum_value: Enum, public sint32: int32) {
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeI64(this.long);
		this.enum_value.serialize(serializer);
		serializer.serializeI32(this.sint32);
	}

	static deserialize(deserializer: Deserializer): InnerInner {
		const long = deserializer.deserializeI64();
		const enum_value = Enum.deserialize(deserializer);
		const sint32 = deserializer.deserializeI32();
		return new InnerInner(long, enum_value, sint32);
	}

}
export class Outer {

	constructor(public bools: Seq<bool>, public double: float64) {
	}

	public serialize(serializer: Serializer): void {
		Helpers.serializeVectorBool(this.bools, serializer);
		serializer.serializeF64(this.double);
	}

	static deserialize(deserializer: Deserializer): Outer {
		const bools = Helpers.deserializeVectorBool(deserializer);
		const double = deserializer.deserializeF64();
		return new Outer(bools, double);
	}

}
export class Test {

	constructor(public string: str, public uint32: uint32, public inner: Inner, public float: float32) {
	}

	public serialize(serializer: Serializer): void {
		serializer.serializeStr(this.string);
		serializer.serializeU32(this.uint32);
		this.inner.serialize(serializer);
		serializer.serializeF32(this.float);
	}

	static deserialize(deserializer: Deserializer): Test {
		const string = deserializer.deserializeStr();
		const uint32 = deserializer.deserializeU32();
		const inner = Inner.deserialize(deserializer);
		const float = deserializer.deserializeF32();
		return new Test(string, uint32, inner, float);
	}

}
export class Helpers {
	static serializeVectorBool(value: Seq<bool>, serializer: Serializer): void {
		serializer.serializeLen(value.length);
		value.forEach((item: bool) => {
			serializer.serializeBool(item);
		});
	}

	static deserializeVectorBool(deserializer: Deserializer): Seq<bool> {
		const length = deserializer.deserializeLen();
		const list: Seq<bool> = [];
		for (let i = 0; i < length; i++) {
			list.push(deserializer.deserializeBool());
		}
		return list;
	}

}

