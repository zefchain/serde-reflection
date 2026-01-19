// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.novi.serde

/**
 * Immutable wrapper class around ByteArray.
 *
 * Enforces value-semantice for `equals` and `hashCode`.
 */
class Bytes private constructor(private val content: ByteArray) {
    fun content(): ByteArray {
        return content.copyOf()
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is Bytes) return false
        return content.contentEquals(other.content)
    }

    override fun hashCode(): Int {
        return content.contentHashCode()
    }

    companion object {
        private val EMPTY = Bytes(ByteArray(0))

        fun empty(): Bytes {
            return EMPTY
        }

        fun valueOf(content: ByteArray): Bytes {
            if (content.isEmpty()) {
                return EMPTY
            }
            return Bytes(content.copyOf())
        }
    }
}
