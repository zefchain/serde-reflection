// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

using System.Numerics;

namespace Serde
{
    public interface ISerializer
    {
        void serialize_str(string value);

        void serialize_vec_bytes(ValueArray<ValueArray<byte>> value);

        void serialize_bytes(ValueArray<byte> value);

        void serialize_bool(bool value);

        void serialize_unit(Unit value);

        void serialize_char(char value);

        void serialize_f32(float value);

        void serialize_f64(double value);

        void serialize_u8(byte value);

        void serialize_u16(ushort value);

        void serialize_u32(uint value);

        void serialize_u64(ulong value);

        void serialize_u128(BigInteger value);

        void serialize_i8(sbyte value);

        void serialize_i16(short value);

        void serialize_i32(int value);

        void serialize_i64(long value);

        void serialize_i128(BigInteger value);

        void serialize_len(long value);

        void serialize_variant_index(int value);

        void serialize_option_tag(bool value);

        void increase_container_depth();

        void decrease_container_depth();

        int get_buffer_offset();

        void sort_map_entries(int[] offsets);

        byte[] get_bytes();
    }
}
