import { BinaryReader, BinaryWriter } from "./serde";

export class BincodeReader extends BinaryReader {
	read_length() {
		return Number(this.read_u64())
	}

	public read_variant_index() {
		return this.read_u32()
	}

	check_that_key_slices_are_increasing(key1: [number, number], key2: [number, number]) {
		return
	}
}

export class BincodeWriter extends BinaryWriter {
	write_length(value: number) {
		this.write_u64(value)
	}

	public write_variant_index(value: number) {
		this.write_u32(value)
	}

	public sort_map_entries(offsets: number[]) {
		return
	}
}
