// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

using System;
using System.IO;
using System.Numerics;
using System.Text;

namespace Serde
{
    public abstract class BinaryDeserializer : IDeserializer, IDisposable
    {
        protected readonly ArraySegment<byte> input;
        protected readonly BinaryReader reader;
        protected readonly Encoding utf8 = Encoding.GetEncoding("utf-8", new EncoderExceptionFallback(), new DecoderExceptionFallback());
        private long containerDepthBudget;

        public BinaryDeserializer(byte[] _input, long maxContainerDepth) : this(new ArraySegment<byte>(_input), maxContainerDepth) { }

        public BinaryDeserializer(ArraySegment<byte> _input, long maxContainerDepth)
        {
            input = _input;
            reader = new BinaryReader(new MemoryStream(input.Array, input.Offset, input.Count));
            containerDepthBudget = maxContainerDepth;
        }

        public void Dispose() => reader.Dispose();

        public int get_buffer_offset() => (int)reader.BaseStream.Position;

        public abstract long deserialize_len();
        public abstract int deserialize_variant_index();
        public abstract void check_that_key_slices_are_increasing(Range key1, Range key2);

        public char deserialize_char() => throw new DeserializationException("Not implemented: char deserialization");

        public float deserialize_f32() => reader.ReadSingle();

        public double deserialize_f64() => reader.ReadDouble();

        public void increase_container_depth()
        {
            if (containerDepthBudget == 0)
            {
                throw new DeserializationException("Exceeded maximum container depth");
            }
            containerDepthBudget -= 1;
        }

        public void decrease_container_depth()
        {
            containerDepthBudget += 1;
        }

        public string deserialize_str()
        {
            long len = deserialize_len();
            if (len < 0 || len > int.MaxValue)
            {
                throw new DeserializationException("Incorrect length value for C# string");
            }
            byte[] content = reader.ReadBytes((int)len);
            if (content.Length < len)
                throw new DeserializationException($"Need {len - content.Length} more bytes for string");
            return utf8.GetString(content);
        }

        public ValueArray<ValueArray<byte>> deserialize_vec_bytes() {
            long len = deserialize_len();
            ValueArray<byte>[] content = new ValueArray<byte>[len];

            if (len < 0 || len > int.MaxValue)
            {
                throw new DeserializationException("Incorrect length value for C# array");
            }

            for (long i = 0; i < len; i++) {
                content[i] = deserialize_bytes();
            }

            return new ValueArray<ValueArray<byte>>(content);
        }

        public ValueArray<byte> deserialize_bytes()
        {
            long len = deserialize_len();
            if (len < 0 || len > int.MaxValue)
            {
                throw new DeserializationException("Incorrect length value for C# array");
            }
            byte[] content = reader.ReadBytes((int)len);
            if (content.Length < len)
                throw new DeserializationException($"Need {len - content.Length} more bytes for byte array");
            return new ValueArray<byte>(content);
        }

        public bool deserialize_bool()
        {
            byte value = reader.ReadByte();
            switch (value)
            {
                case 0: return false;
                case 1: return true;
                default: throw new DeserializationException("Incorrect value for bool: " + value);
            }
        }

        public Unit deserialize_unit() => new Unit();

        public byte deserialize_u8() => reader.ReadByte();

        public ushort deserialize_u16() => reader.ReadUInt16();

        public uint deserialize_u32() => reader.ReadUInt32();

        public ulong deserialize_u64() => reader.ReadUInt64();

        public BigInteger deserialize_u128()
        {
            BigInteger signed = deserialize_i128();
            if (signed >= 0)
            {
                return signed;
            }
            else
            {
                return signed + (BigInteger.One << 128);
            }
        }

        public sbyte deserialize_i8() => reader.ReadSByte();

        public short deserialize_i16() => reader.ReadInt16();

        public int deserialize_i32() => reader.ReadInt32();

        public long deserialize_i64() => reader.ReadInt64();

        public BigInteger deserialize_i128()
        {
            byte[] content = reader.ReadBytes(16);
            if (content.Length < 16)
                throw new DeserializationException("Need more bytes to deserialize 128-bit integer");
            return new BigInteger(content);
        }

        public bool deserialize_option_tag()
        {
            byte value = reader.ReadByte();
            switch (value)
            {
                case 0: return false;
                case 1: return true;
                default: throw new DeserializationException("Incorrect value for Option tag: " + value);
            }
        }
    }
}
