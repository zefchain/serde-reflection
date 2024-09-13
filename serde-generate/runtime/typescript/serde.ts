export type Optional<T> = T | null
export type Seq<T> = T[]
export type Tuple<T extends any[]> = T
export type ListTuple<T extends any[]> = Tuple<T>[]
export type Map<K, V> = globalThis.Map<K, V>

export type unit = null
export type bool = boolean
export type i8 = number
export type i16 = number
export type i32 = number
export type i64 = bigint
export type i128 = bigint
export type u8 = number
export type u16 = number
export type u32 = number
export type u64 = bigint
export type u128 = bigint
export type f32 = number
export type f64 = number
export type char = string
export type str = string
export type bytes = Uint8Array

export type WrapperOfCase<T extends { $: string }, K = T["$"]> = T extends { $: infer _U extends K } ? T : never

export interface Reader {
	readString(): string
	readBool(): boolean
	readUnit(): null
	readChar(): string
	readF32(): number
	readF64(): number
	readU8(): number
	readU16(): number
	readU32(): number
	readU64(): bigint
	readU128(): bigint
	readI8(): number
	readI16(): number
	readI32(): number
	readI64(): bigint
	readI128(): bigint
	readLength(): number
	readVariantIndex(): number
	readOptionTag(): boolean
	readList<T>(readFn: () => T, length?: number): T[]
	readMap<K, V>(readKey: () => K, readValue: () => V): Map<K, V>
	checkThatKeySlicesAreIncreasing(key1: [number, number], key2: [number, number]): void
}

export interface Writer {
	writeString(value: string): void
	writeBool(value: boolean): void
	writeUnit(value: null): void
	writeChar(value: string): void
	writeF32(value: number): void
	writeF64(value: number): void
	writeU8(value: number): void
	writeU16(value: number): void
	writeU32(value: number): void
	writeU64(value: bigint | number): void
	writeU128(value: bigint | number): void
	writeI8(value: number): void
	writeI16(value: number): void
	writeI32(value: number): void
	writeI64(value: bigint | number): void
	writeI128(value: bigint | number): void
	writeLength(value: number): void
	writeVariantIndex(value: number): void
	writeOptionTag(value: boolean): void
	writeMap<K, V>(value: Map<K, V>, writeKey: (key: K) => void, writeValue: (value: V) => void): void
	getBytes(): Uint8Array
	sortMapEntries(offsets: number[]): void
}

const BIG_32 = 32n
const BIG_64 = 64n
const BIG_32Fs = 429967295n
const BIG_64Fs = 18446744073709551615n

let WRITE_HEAP = new DataView(new ArrayBuffer(128))

export abstract class BinaryWriter implements Writer {
	public static readonly TEXT_ENCODER = new TextEncoder()

	public view = WRITE_HEAP
	public offset = 0

	constructor() {
		if (WRITE_HEAP.byteLength > 1024) {
			this.view = WRITE_HEAP = new DataView(new ArrayBuffer(128))
		}
	}

	private alloc(allocLength: number) {
		const wishSize = this.offset + allocLength

		const currentLength = this.view.buffer.byteLength
		if (wishSize > currentLength) {
			let newBufferLength = currentLength
			while (newBufferLength <= wishSize) newBufferLength = newBufferLength << 1

			// TODO: there is new API for resizing buffer, but in Node it seems to be slower then allocating new
			// this.buffer.resize(newBufferLength)

			const newBuffer = new Uint8Array(newBufferLength)
			newBuffer.set(new Uint8Array(this.view.buffer))

			this.view = WRITE_HEAP = new DataView(newBuffer.buffer)
		}
	}

	abstract writeLength(value: number): void
	abstract writeVariantIndex(value: number): void
	abstract sortMapEntries(offsets: number[]): void

	public writeString(value: string) {
		const length = value.length
		// char and U64 for length
		this.alloc(8 + length)

		// encode into buffer with space for string length (u64)
		BinaryWriter.TEXT_ENCODER.encodeInto(value, new Uint8Array(this.view.buffer, this.offset + 8))

		const bLength = BigInt(length)
		this.view.setUint32(this.offset, Number(bLength & BIG_32Fs), true)
		this.view.setUint32(this.offset + 4, Number(bLength >> BIG_32), true)
		this.offset += (8 + length)
	}

	public writeBool(value: boolean) {
		this.writeU8(value ? 1 : 0)
	}

	// eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/explicit-module-boundary-types
	public writeUnit(_value: null) {
		return
	}

	public writeU8(value: number) {
		this.alloc(1)
		this.view.setUint8(this.offset, value)
		this.offset += 1
	}

	public writeU16(value: number) {
		this.alloc(2)
		this.view.setUint16(this.offset, value, true)
		this.offset += 2
	}

	public writeU32(value: number) {
		this.alloc(4)
		this.view.setUint32(this.offset, value, true)
		this.offset += 4
	}

	public writeU64(value: bigint | number) {
		const low = BigInt(value) & BIG_32Fs, high = BigInt(value) >> BIG_32

		this.alloc(8)

		// write little endian number
		this.view.setUint32(this.offset, Number(low), true)
		this.view.setUint32(this.offset + 4, Number(high), true)

		this.offset += 8
	}

	public writeU128(value: bigint | number) {
		const low = BigInt(value) & BIG_64Fs, high = BigInt(value) >> BIG_64

		// write little endian number
		this.writeU64(low)
		this.writeU64(high)
	}

	public writeI8(value: number) {
		this.alloc(1)
		this.view.setInt8(this.offset, value)
		this.offset += 1
	}

	public writeI16(value: number) {
		this.alloc(2)
		this.view.setInt16(this.offset, value, true)
		this.offset += 2
	}

	public writeI32(value: number) {
		this.alloc(4)
		this.view.setInt32(this.offset, value, true)
		this.offset += 4
	}

	public writeI64(value: bigint | number) {
		const low = BigInt(value) & BIG_32Fs, high = BigInt(value) >> BIG_32

		this.alloc(8)

		// write little endian number
		this.view.setInt32(this.offset, Number(low), true)
		this.view.setInt32(this.offset + 4, Number(high), true)

		this.offset += 8
	}

	public writeI128(value: bigint | number) {
		const low = BigInt(value) & BIG_64Fs, high = BigInt(value) >> BIG_64

		// write little endian number
		this.writeI64(low)
		this.writeI64(high)
	}

	public writeOptionTag(value: boolean) {
		this.writeBool(value)
	}

	public writeMap<T, V>(map: Map<T, V>, writeKey: (key: T) => void, writeValue: (value: V) => void): void {
		this.writeLength(map.size)
		const offsets: number[] = []
		for (const [k, v] of map.entries()) {
			offsets.push(this.offset)
			writeKey(k)
			writeValue(v)
		}
		this.sortMapEntries(offsets)
	}

	public writeF32(value: number) {
		this.alloc(4)
		this.view.setFloat32(this.offset, value, true)
		this.offset += 4
	}

	public writeF64(value: number) {
		this.alloc(8)
		this.view.setFloat64(this.offset, value, true)
		this.offset += 8
	}

	public writeChar(_value: string) {
		throw new Error("Method serializeChar not implemented.")
	}

	public getBytes() {
		return new Uint8Array(this.view.buffer).subarray(0, this.offset)
	}
}

export abstract class BinaryReader implements Reader {
	private static readonly TEXT_DECODER = new TextDecoder()

	public offset = 0
	public view: DataView

	constructor(data: Uint8Array) {
		this.view = new DataView(data.buffer)
	}

	abstract readLength(): number
	abstract readVariantIndex(): number
	abstract checkThatKeySlicesAreIncreasing(key1: [number, number], key2: [number, number]): void

	public readString() {
		const length = this.readLength()
		const decoded = BinaryReader.TEXT_DECODER.decode(new Uint8Array(this.view.buffer, this.offset, length))
		this.offset += length
		return decoded
	}

	public readBool() {
		return this.readU8() === 1
	}

	public readUnit() {
		return null
	}

	public readU8() {
		const value = this.view.getUint8(this.offset)
		this.offset += 1
		return value
	}

	public readU16() {
		const value = this.view.getUint16(this.offset, true)
		this.offset += 2
		return value
	}

	public readU32() {
		const value = this.view.getUint32(this.offset, true)
		this.offset += 4
		return value
	}

	public readU64() {
		const low = this.readU32(), high = this.readU32()
		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BIG_32) | BigInt(low)
	}

	public readU128() {
		const low = this.readU64(), high = this.readU64()
		// combine the two 64-bit values and return (little endian)
		return (high << BIG_64) | low
	}

	public readI8() {
		const value = this.view.getInt8(this.offset)
		this.offset += 1
		return value
	}

	public readI16() {
		const value = this.view.getInt16(this.offset, true)
		this.offset += 2
		return value
	}

	public readI32() {
		const value = this.view.getInt32(this.offset, true)
		this.offset += 4
		return value
	}

	public readI64() {
		const low = this.readI32(), high = this.readI32()
		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BIG_32) | BigInt(low)
	}

	public readI128() {
		const low = this.readI64(), high = this.readI64()
		// combine the two 64-bit values and return (little endian)
		return (high << BIG_64) | low
	}

	public readOptionTag = this.readBool

	public readList<T>(readFn: () => T, listLength?: number) {
		const length = listLength ?? this.readLength(), list = new Array<T>(length)
		for (let i = 0; i < length; i++) list[i] = readFn()
		return list
	}

	public readMap<K, V>(readKey: () => K, readValue: () => V) {
		const length = this.readLength(), obj = new Map<K, V>()
		let previousKeyStart = 0, previousKeyEnd = 0
		for (let i = 0; i < length; i++) {
			const keyStart = this.offset, key = readKey(), keyEnd = this.offset
			if (i > 0) {
				this.checkThatKeySlicesAreIncreasing([previousKeyStart, previousKeyEnd], [keyStart, keyEnd])
			}
			previousKeyStart = keyStart
			previousKeyEnd = keyEnd
			obj.set(key, readValue())
		}
		return obj
	}

	public readChar(): string {
		throw new Error("Method readChar not implemented.")
	}

	public readF32() {
		const value = this.view.getFloat32(this.offset, true)
		this.offset += 4
		return value
	}

	public readF64() {
		const value = this.view.getFloat64(this.offset, true)
		this.offset += 8
		return value
	}
}
