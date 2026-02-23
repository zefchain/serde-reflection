// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.bcs

import com.novi.serde.Int128
import com.novi.serde.UInt128

fun expect(condition: Boolean, message: String) {
    if (!condition) {
        throw RuntimeException(message)
    }
}

fun test_serialize_u128() {
    var serializer = BcsSerializer()
    serializer.serialize_u128(UInt128(ULong.MAX_VALUE, ULong.MAX_VALUE))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1)), "max u128")

    serializer = BcsSerializer()
    serializer.serialize_u128(UInt128(0uL, 1uL))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)), "u128 one")

    serializer = BcsSerializer()
    serializer.serialize_u128(UInt128(0uL, 0uL))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)), "u128 zero")
}

fun test_serialize_i128() {
    var serializer = BcsSerializer()
    serializer.serialize_i128(Int128(-1L, ULong.MAX_VALUE))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1)), "i128 minus one")

    serializer = BcsSerializer()
    serializer.serialize_i128(Int128(0L, 1uL))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)), "i128 one")

    serializer = BcsSerializer()
    serializer.serialize_i128(Int128(Long.MAX_VALUE, ULong.MAX_VALUE))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 127)), "i128 max")

    serializer = BcsSerializer()
    serializer.serialize_i128(Int128(Long.MIN_VALUE, 0uL))
    expect(serializer.get_bytes().contentEquals(byteArrayOf(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -128)), "i128 min")
}

fun test_serializer_slice_ordering() {
    val serializer = BcsSerializer()

    serializer.serialize_u8(0xFFu.toUByte())
    serializer.serialize_u32(1u)
    serializer.serialize_u32(1u)
    serializer.serialize_u32(2u)
    expect(serializer.get_bytes().contentEquals(byteArrayOf(-1, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0)), "before sort")

    val offsets = intArrayOf(1, 2, 4, 7, 8, 9)
    serializer.sort_map_entries(offsets)
    expect(serializer.get_bytes().contentEquals(byteArrayOf(-1, 0, 0, 0, 0, 0, 1, 0, 1, 2, 0, 0, 0)), "after sort")
}

fun main() {
    test_serialize_u128()
    test_serialize_i128()
    test_serializer_slice_ordering()
}
