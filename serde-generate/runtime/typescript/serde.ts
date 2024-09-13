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
	readBytes(): Uint8Array
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
	writeBytes(value: Uint8Array): void
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


export abstract class BinaryWriter implements Writer {
	public static readonly BIG_32: bigint = 32n
	public static readonly BIG_64: bigint = 64n
	public static readonly BIG_32Fs: bigint = 429967295n
	public static readonly BIG_64Fs: bigint = 18446744073709551615n
	public static readonly textEncoder = new TextEncoder()

	public buffer = new ArrayBuffer(64)
	public offset = 0


	private ensureBufferWillHandleSize(bytes: number) {
		const wishSize = this.offset + bytes
		if (wishSize > this.buffer.byteLength) {
			let newBufferLength = this.buffer.byteLength
			while (newBufferLength < wishSize) newBufferLength *= 2
			newBufferLength = Math.max(wishSize, newBufferLength)

			// TODO: there is new API for resizing buffer, but in Node it seems to be slower then allocating new
			// this.buffer.resize(newBufferLength)
			const newBuffer = new ArrayBuffer(newBufferLength)
			new Uint8Array(newBuffer).set(new Uint8Array(this.buffer))
			this.buffer = newBuffer
		}
	}

	protected write(values: Uint8Array) {
		this.ensureBufferWillHandleSize(values.length)
		new Uint8Array(this.buffer, this.offset).set(values)
		this.offset += values.length
	}

	abstract writeLength(value: number): void
	abstract writeVariantIndex(value: number): void
	abstract sortMapEntries(offsets: number[]): void

	public writeString(value: string) {
		const bytes = value.length * 3 + 8
		this.ensureBufferWillHandleSize(bytes)
		// TODO: check this for correctness
		const { written } = BinaryWriter.textEncoder.encodeInto(value, new Uint8Array(this.buffer, this.offset + 8))
		this.writeU64(written)
		this.offset += written
	}

	public writeBytes(value: Uint8Array) {
		this.writeLength(value.length)
		this.write(value)
	}

	public writeBool(value: boolean) {
		const byteValue = value ? 1 : 0
		this.write(new Uint8Array([byteValue]))
	}

	// eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/explicit-module-boundary-types
	public writeUnit(_value: null) {
		return
	}

	private writeWithFunction(fn: (byteOffset: number, value: number, littleEndian: boolean) => void, bytesLength: number, value: number) {
		this.ensureBufferWillHandleSize(bytesLength)
		const dv = new DataView(this.buffer, this.offset)
		fn.apply(dv, [0, value, true])
		this.offset += bytesLength
	}

	public writeU8(value: number) {
		this.write(new Uint8Array([value]))
	}

	public writeU16(value: number) {
		this.writeWithFunction(DataView.prototype.setUint16, 2, value)
	}

	public writeU32(value: number) {
		this.writeWithFunction(DataView.prototype.setUint32, 4, value)
	}

	public writeU64(value: bigint | number) {
		const low = BigInt(value) & BinaryWriter.BIG_32Fs
		const high = BigInt(value) >> BinaryWriter.BIG_32

		// write little endian number
		this.writeU32(Number(low))
		this.writeU32(Number(high))
	}

	public writeU128(value: bigint | number) {
		const low = BigInt(value) & BinaryWriter.BIG_64Fs
		const high = BigInt(value) >> BinaryWriter.BIG_64

		// write little endian number
		this.writeU64(low)
		this.writeU64(high)
	}

	public writeI8(value: number) {
		const bytes = 1
		this.ensureBufferWillHandleSize(bytes)
		new DataView(this.buffer, this.offset).setInt8(0, value)
		this.offset += bytes
	}

	public writeI16(value: number) {
		const bytes = 2
		this.ensureBufferWillHandleSize(bytes)
		new DataView(this.buffer, this.offset).setInt16(0, value, true)
		this.offset += bytes
	}

	public writeI32(value: number) {
		const bytes = 4
		this.ensureBufferWillHandleSize(bytes)
		new DataView(this.buffer, this.offset).setInt32(0, value, true)
		this.offset += bytes
	}

	public writeI64(value: bigint | number) {
		const low = BigInt(value) & BinaryWriter.BIG_32Fs
		const high = BigInt(value) >> BinaryWriter.BIG_32

		// write little endian number
		this.writeI32(Number(low))
		this.writeI32(Number(high))
	}

	public writeI128(value: bigint | number) {
		const low = BigInt(value) & BinaryWriter.BIG_64Fs
		const high = BigInt(value) >> BinaryWriter.BIG_64

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
		const bytes = 4
		this.ensureBufferWillHandleSize(bytes)
		new DataView(this.buffer, this.offset).setFloat32(0, value, true)
		this.offset += bytes
	}

	public writeF64(value: number) {
		const bytes = 8
		this.ensureBufferWillHandleSize(bytes)
		new DataView(this.buffer, this.offset).setFloat64(0, value, true)
		this.offset += bytes
	}

	public writeChar(_value: string) {
		throw new Error("Method serializeChar not implemented.")
	}

	public getBytes() {
		return new Uint8Array(this.buffer).slice(0, this.offset)
	}
}

export abstract class BinaryReader implements Reader {
	private static readonly BIG_32: bigint = 32n
	private static readonly BIG_64: bigint = 64n
	private static readonly textDecoder = new TextDecoder()

	public buffer: ArrayBuffer
	public offset = 0

	constructor(data: Uint8Array) {
		// copies data to prevent outside mutation of buffer.
		this.buffer = new ArrayBuffer(data.length)
		new Uint8Array(this.buffer).set(data, 0)
	}

	private read(length: number) {
		const bytes = this.buffer.slice(this.offset, this.offset + length)
		this.offset += length
		return bytes
	}

	abstract readLength(): number
	abstract readVariantIndex(): number
	abstract checkThatKeySlicesAreIncreasing(key1: [number, number], key2: [number, number]): void

	public readString() {
		const value = this.readBytes()
		return BinaryReader.textDecoder.decode(value)
	}

	public readBytes() {
		const len = this.readLength()
		if (len < 0) {
			throw new Error("Length of a bytes array can't be negative")
		}
		return new Uint8Array(this.read(len))
	}

	public readBool() {
		const bool = new Uint8Array(this.read(1))[0]
		return bool == 1
	}

	public readUnit() {
		return null
	}

	public readU8() {
		return new DataView(this.read(1)).getUint8(0)
	}

	public readU16() {
		return new DataView(this.read(2)).getUint16(0, true)
	}

	public readU32() {
		return new DataView(this.read(4)).getUint32(0, true)
	}

	public readU64() {
		const low = this.readU32()
		const high = this.readU32()

		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BinaryReader.BIG_32) | BigInt(low)
	}

	public readU128() {
		const low = this.readU64()
		const high = this.readU64()

		// combine the two 64-bit values and return (little endian)
		return (high << BinaryReader.BIG_64) | low
	}

	public readI8() {
		return new DataView(this.read(1)).getInt8(0)
	}

	public readI16() {
		return new DataView(this.read(2)).getInt16(0, true)
	}

	public readI32() {
		return new DataView(this.read(4)).getInt32(0, true)
	}

	public readI64() {
		const low = this.readI32()
		const high = this.readI32()

		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BinaryReader.BIG_32) | BigInt(low)
	}

	public readI128() {
		const low = this.readI64()
		const high = this.readI64()

		// combine the two 64-bit values and return (little endian)
		return (BigInt(high) << BinaryReader.BIG_64) | BigInt(low)
	}

	public readOptionTag() {
		return this.readBool()
	}

	public readList<T>(readFn: () => T, listLength?: number) {
		const length = listLength ?? this.readLength()
		const list = new Array<T>(length)
		for (let i = 0; i < length; i++) list[i] = readFn()
		return list
	}

	public readMap<K, V>(readKey: () => K, readValue: () => V) {
		const length = this.readLength(), obj = new Map<K, V>()
		let previousKeyStart = 0, previousKeyEnd = 0
		for (let i = 0; i < length; i++) {
			const keyStart = this.offset,
				key = readKey(),
				keyEnd = this.offset
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
		return new DataView(this.read(4)).getFloat32(0, true)
	}

	public readF64() {
		return new DataView(this.read(8)).getFloat64(0, true)
	}
}
