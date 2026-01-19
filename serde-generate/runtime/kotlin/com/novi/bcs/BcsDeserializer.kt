// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.bcs

import com.novi.serde.BinaryDeserializer
import com.novi.serde.DeserializationError
import com.novi.serde.Slice

class BcsDeserializer(input: ByteArray) : BinaryDeserializer(input, BcsSerializer.MAX_CONTAINER_DEPTH) {
    @Throws(DeserializationError::class)
    override fun deserialize_f32(): Float {
        throw DeserializationError("Not implemented: deserialize_f32")
    }

    @Throws(DeserializationError::class)
    override fun deserialize_f64(): Double {
        throw DeserializationError("Not implemented: deserialize_f64")
    }

    @Throws(DeserializationError::class)
    private fun deserialize_uleb128_as_u32(): Int {
        var value = 0L
        var shift = 0
        while (shift < 32) {
            val x = getByte().toInt() and 0xff
            val digit = x and 0x7f
            value = value or (digit.toLong() shl shift)
            if (value < 0 || value > Int.MAX_VALUE.toLong()) {
                throw DeserializationError("Overflow while parsing uleb128-encoded uint32 value")
            }
            if (digit == x) {
                if (shift > 0 && digit == 0) {
                    throw DeserializationError("Invalid uleb128 number (unexpected zero digit)")
                }
                return value.toInt()
            }
            shift += 7
        }
        throw DeserializationError("Overflow while parsing uleb128-encoded uint32 value")
    }

    @Throws(DeserializationError::class)
    override fun deserialize_len(): Long {
        return deserialize_uleb128_as_u32().toLong()
    }

    @Throws(DeserializationError::class)
    override fun deserialize_variant_index(): Int {
        return deserialize_uleb128_as_u32()
    }

    @Throws(DeserializationError::class)
    override fun check_that_key_slices_are_increasing(key1: Slice, key2: Slice) {
        if (Slice.compare_bytes(input, key1, key2) >= 0) {
            throw DeserializationError("Error while decoding map: keys are not serialized in the expected order")
        }
    }
}
