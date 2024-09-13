import { BinaryReader, BinaryWriter } from "./serde.ts";

export class BincodeReader extends BinaryReader {
	readLength() {
		return Number(this.readU64())
	}

	public readVariantIndex() {
		return this.readU32()
	}

	checkThatKeySlicesAreIncreasing(key1: [number, number], key2: [number, number]) {
		return
	}
}

export class BincodeWriter extends BinaryWriter {
	writeLength(value: number) {
		this.writeU64(value)
	}

	public writeVariantIndex(value: number) {
		this.writeU32(value)
	}

	public sortMapEntries(offsets: number[]) {
		return
	}
}
