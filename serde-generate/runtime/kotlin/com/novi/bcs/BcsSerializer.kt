// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.bcs

import com.novi.serde.BinarySerializer
import com.novi.serde.SerializationError
import com.novi.serde.Slice

class BcsSerializer : BinarySerializer(MAX_CONTAINER_DEPTH) {
    @Throws(SerializationError::class)
    override fun serialize_f32(value: Float) {
        throw SerializationError("Not implemented: serialize_f32")
    }

    @Throws(SerializationError::class)
    override fun serialize_f64(value: Double) {
        throw SerializationError("Not implemented: serialize_f64")
    }

    private fun serialize_u32_as_uleb128(value: Int) {
        var v = value
        while ((v ushr 7) != 0) {
            output.writeByte(((v and 0x7f) or 0x80).toByte())
            v = v ushr 7
        }
        output.writeByte(v.toByte())
    }

    @Throws(SerializationError::class)
    override fun serialize_len(value: Long) {
        if (value < 0 || value > MAX_LENGTH) {
            throw SerializationError("Incorrect length value")
        }
        serialize_u32_as_uleb128(value.toInt())
    }

    @Throws(SerializationError::class)
    override fun serialize_variant_index(value: Int) {
        serialize_u32_as_uleb128(value)
    }

    override fun sort_map_entries(offsets: IntArray) {
        if (offsets.size <= 1) {
            return
        }
        val offset0 = offsets[0]
        val content = output.getBuffer()
        val slices = Array(offsets.size) { index ->
            if (index < offsets.size - 1) {
                Slice(offsets[index], offsets[index + 1])
            } else {
                Slice(offsets[index], output.size())
            }
        }

        slices.sortWith { slice1, slice2 ->
            Slice.compare_bytes(content, slice1, slice2)
        }

        val totalLength = output.size() - offset0
        val oldContent = ByteArray(totalLength)
        content.copyInto(oldContent, startIndex = offset0, endIndex = offset0 + totalLength)

        var position = offset0
        for (slice in slices) {
            val start = slice.start
            val end = slice.end
            val length = end - start
            oldContent.copyInto(content, destinationOffset = position, startIndex = start - offset0, endIndex = start - offset0 + length)
            position += length
        }
    }

    companion object {
        const val MAX_LENGTH: Long = Int.MAX_VALUE.toLong()
        const val MAX_CONTAINER_DEPTH: Long = 500
    }
}
