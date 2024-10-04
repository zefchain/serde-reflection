export type Optional<T> = T | null
export type Seq<T> = T[]
export type Tuple<T extends any[]> = {
	[K in keyof T as `$${Exclude<K, keyof any[]> extends string ? Exclude<K, keyof any[]> : never}` ]: T[K]
}
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

export interface Reader {
	read_string(): string
	read_bool(): boolean
	read_unit(): null
	read_char(): string
	read_f32(): number
	read_f64(): number
	read_u8(): number
	read_u16(): number
	read_u32(): number
	read_u64(): bigint
	read_u128(): bigint
	read_i8(): number
	read_i16(): number
	read_i32(): number
	read_i64(): bigint
	read_i128(): bigint
	read_length(): number
	read_variant_index(): number
	read_option_tag(): boolean
	read_list<T>(read_fn: () => T, length?: number): T[]
	read_map<K, V>(read_key: () => K, read_value: () => V): Map<K, V>
	check_that_key_slices_are_increasing(key1: [number, number], key2: [number, number]): void
}

export interface Writer {
	write_string(value: string): void
	write_bool(value: boolean): void
	write_unit(value: null): void
	write_char(value: string): void
	write_f32(value: number): void
	write_f64(value: number): void
	write_u8(value: number): void
	write_u16(value: number): void
	write_u32(value: number): void
	write_u64(value: bigint | number): void
	write_u128(value: bigint | number): void
	write_i8(value: number): void
	write_i16(value: number): void
	write_i32(value: number): void
	write_i64(value: bigint | number): void
	write_i128(value: bigint | number): void
	write_length(value: number): void
	write_variant_index(value: number): void
	write_option_tag(value: boolean): void
	write_map<K, V>(value: Map<K, V>, write_key: (key: K) => void, write_value: (value: V) => void): void
	get_bytes(): Uint8Array
	sort_map_entries(offsets: number[]): void
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

	abstract write_length(value: number): void
	abstract write_variant_index(value: number): void
	abstract sort_map_entries(offsets: number[]): void

	public write_string(value: string) {
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

	public write_bool(value: boolean) {
		this.write_u8(value ? 1 : 0)
	}

	// eslint-disable-next-line @typescript-eslint/no-unused-vars,@typescript-eslint/explicit-module-boundary-types
	public write_unit(_value: null) {
		return
	}

	public write_u8(value: number) {
		this.alloc(1)
		this.view.setUint8(this.offset, value)
		this.offset += 1
	}

	public write_u16(value: number) {
		this.alloc(2)
		this.view.setUint16(this.offset, value, true)
		this.offset += 2
	}

	public write_u32(value: number) {
		this.alloc(4)
		this.view.setUint32(this.offset, value, true)
		this.offset += 4
	}

	public write_u64(value: bigint | number) {
		const low = BigInt(value) & BIG_32Fs, high = BigInt(value) >> BIG_32

		this.alloc(8)

		// write little endian number
		this.view.setUint32(this.offset, Number(low), true)
		this.view.setUint32(this.offset + 4, Number(high), true)

		this.offset += 8
	}

	public write_u128(value: bigint | number) {
		const low = BigInt(value) & BIG_64Fs, high = BigInt(value) >> BIG_64

		// write little endian number
		this.write_u64(low)
		this.write_u64(high)
	}

	public write_i8(value: number) {
		this.alloc(1)
		this.view.setInt8(this.offset, value)
		this.offset += 1
	}

	public write_i16(value: number) {
		this.alloc(2)
		this.view.setInt16(this.offset, value, true)
		this.offset += 2
	}

	public write_i32(value: number) {
		this.alloc(4)
		this.view.setInt32(this.offset, value, true)
		this.offset += 4
	}

	public write_i64(value: bigint | number) {
		const low = BigInt(value) & BIG_32Fs, high = BigInt(value) >> BIG_32

		this.alloc(8)

		// write little endian number
		this.view.setInt32(this.offset, Number(low), true)
		this.view.setInt32(this.offset + 4, Number(high), true)

		this.offset += 8
	}

	public write_i128(value: bigint | number) {
		const low = BigInt(value) & BIG_64Fs, high = BigInt(value) >> BIG_64

		// write little endian number
		this.write_i64(low)
		this.write_i64(high)
	}

	public write_option_tag(value: boolean) {
		this.write_bool(value)
	}

	public write_map<T, V>(map: Map<T, V>, write_key: (key: T) => void, write_value: (value: V) => void): void {
		this.write_length(map.size)
		const offsets: number[] = []
		for (const [k, v] of map.entries()) {
			offsets.push(this.offset)
			write_key(k)
			write_value(v)
		}
		this.sort_map_entries(offsets)
	}

	public write_f32(value: number) {
		this.alloc(4)
		this.view.setFloat32(this.offset, value, true)
		this.offset += 4
	}

	public write_f64(value: number) {
		this.alloc(8)
		this.view.setFloat64(this.offset, value, true)
		this.offset += 8
	}

	public write_char(_value: string) {
		throw new Error("Method serializeChar not implemented.")
	}

	public get_bytes() {
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

	abstract read_length(): number
	abstract read_variant_index(): number
	abstract check_that_key_slices_are_increasing(key1: [number, number], key2: [number, number]): void

	public read_string() {
		const length = this.read_length()
		const decoded = BinaryReader.TEXT_DECODER.decode(new Uint8Array(this.view.buffer, this.offset, length))
		this.offset += length
		return decoded
	}

	public read_bool() {
		return this.read_u8() === 1
	}

	public read_unit() {
		return null
	}

	public read_u8() {
		const value = this.view.getUint8(this.offset)
		this.offset += 1
		return value
	}

	public read_u16() {
		const value = this.view.getUint16(this.offset, true)
		this.offset += 2
		return value
	}

	public read_u32() {
		const value = this.view.getUint32(this.offset, true)
		this.offset += 4
		return value
	}

	public read_u64() {
		const low = this.read_u32(), high = this.read_u32()
		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BIG_32) | BigInt(low)
	}

	public read_u128() {
		const low = this.read_u64(), high = this.read_u64()
		// combine the two 64-bit values and return (little endian)
		return (high << BIG_64) | low
	}

	public read_i8() {
		const value = this.view.getInt8(this.offset)
		this.offset += 1
		return value
	}

	public read_i16() {
		const value = this.view.getInt16(this.offset, true)
		this.offset += 2
		return value
	}

	public read_i32() {
		const value = this.view.getInt32(this.offset, true)
		this.offset += 4
		return value
	}

	public read_i64() {
		const low = this.read_i32(), high = this.read_i32()
		// combine the two 32-bit values and return (little endian)
		return (BigInt(high) << BIG_32) | BigInt(low)
	}

	public read_i128() {
		const low = this.read_i64(), high = this.read_i64()
		// combine the two 64-bit values and return (little endian)
		return (high << BIG_64) | low
	}

	public read_option_tag = this.read_bool

	public read_list<T>(read_fn: () => T, listLength?: number) {
		const length = listLength ?? this.read_length(), list = new Array<T>(length)
		for (let i = 0; i < length; i++) list[i] = read_fn()
		return list
	}

	public read_map<K, V>(read_key: () => K, read_value: () => V) {
		const length = this.read_length(), obj = new Map<K, V>()
		let previousKeyStart = 0, previousKeyEnd = 0
		for (let i = 0; i < length; i++) {
			const keyStart = this.offset, key = read_key(), keyEnd = this.offset
			if (i > 0) {
				this.check_that_key_slices_are_increasing([previousKeyStart, previousKeyEnd], [keyStart, keyEnd])
			}
			previousKeyStart = keyStart
			previousKeyEnd = keyEnd
			obj.set(key, read_value())
		}
		return obj
	}

	public read_char(): string {
		throw new Error("Method read_char not implemented.")
	}

	public read_f32() {
		const value = this.view.getFloat32(this.offset, true)
		this.offset += 4
		return value
	}

	public read_f64() {
		const value = this.view.getFloat64(this.offset, true)
		this.offset += 8
		return value
	}
}
