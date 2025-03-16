# Copyright (c) Facebook, Inc. and its affiliates
# SPDX-License-Identifier: MIT OR Apache-2.0

import unittest
import serde_types as st
import bcs
import typing
import sys
from dataclasses import dataclass
from collections import OrderedDict


def encode_u32_as_uleb128(value: int) -> bytes:
    serializer = bcs.BcsSerializer()
    serializer.serialize_u32_as_uleb128(value)
    return serializer.get_buffer()


def decode_uleb128_as_u32(content: bytes) -> int:
    deserializer = bcs.BcsDeserializer(content)
    value = deserializer.deserialize_uleb128_as_u32()
    assert deserializer.get_remaining_buffer() == b""
    return value


def encode_length(value: int) -> bytes:
    serializer = bcs.BcsSerializer()
    serializer.serialize_len(value)
    return serializer.get_buffer()


def decode_length(content: bytes) -> int:
    deserializer = bcs.BcsDeserializer(content)
    value = deserializer.deserialize_len()
    assert deserializer.get_remaining_buffer() == b""
    return value


class BcsTestCase(unittest.TestCase):
    def test_bcs_bool(self):
        self.assertEqual(bcs.serialize(False, bool), b"\x00")
        self.assertEqual(bcs.serialize(True, bool), b"\x01")
        self.assertEqual(bcs.deserialize(b"\x00", bool), (False, b""))
        self.assertEqual(bcs.deserialize(b"\x01", bool), (True, b""))
        with self.assertRaises(st.DeserializationError):
            bcs.deserialize(b"\x02", bool)
        with self.assertRaises(st.DeserializationError):
            bcs.deserialize(b"", bool)

    def test_bcs_u8(self):
        self.assertEqual(bcs.serialize(0x1, st.uint8), b"\x01")
        self.assertEqual(bcs.deserialize(b"\xff", st.uint8), (255, b""))

    def test_bcs_u16(self):
        self.assertEqual(bcs.serialize(0x0102, st.uint16), b"\x02\x01")
        self.assertEqual(bcs.deserialize(b"\xff\xff", st.uint16), (65535, b""))

    def test_bcs_u32(self):
        self.assertEqual(bcs.serialize(0x01020304, st.uint32), b"\x04\x03\x02\x01")
        self.assertEqual(
            bcs.deserialize(b"\xff\xff\xff\xff", st.uint32), (4294967295, b"")
        )

    def test_bcs_u64(self):
        self.assertEqual(
            bcs.serialize(0x0102030405060708, st.uint64),
            b"\x08\x07\x06\x05\x04\x03\x02\x01",
        )
        self.assertEqual(bcs.deserialize(b"\xff" * 8, st.uint64), ((1 << 64) - 1, b""))

    def test_bcs_u128(self):
        self.assertEqual(
            bcs.serialize(st.uint128(0x0102030405060708090A0B0C0D0E0F10), st.uint128),
            b"\x10\x0f\x0e\r\x0c\x0b\n\t\x08\x07\x06\x05\x04\x03\x02\x01",
        )
        self.assertEqual(
            bcs.deserialize(b"\xff" * 16, st.uint128), (st.uint128((1 << 128) - 1), b"")
        )

    def test_bcs_i8(self):
        self.assertEqual(bcs.serialize(4, st.int8), b"\x04")
        self.assertEqual(bcs.serialize(-2, st.int8), b"\xfe")
        self.assertEqual(bcs.deserialize(b"\xff", st.int8), (-1, b""))

    def test_bcs_i16(self):
        self.assertEqual(bcs.serialize(0x0102, st.int16), b"\x02\x01")
        self.assertEqual(bcs.deserialize(b"\xff\xff", st.int16), (-1, b""))

    def test_bcs_i32(self):
        self.assertEqual(bcs.serialize(0x01020304, st.int32), b"\x04\x03\x02\x01")
        self.assertEqual(bcs.deserialize(b"\xff\xff\xff\xff", st.int32), (-1, b""))

    def test_bcs_i64(self):
        self.assertEqual(
            bcs.serialize(0x0102030405060708, st.int64),
            b"\x08\x07\x06\x05\x04\x03\x02\x01",
        )
        self.assertEqual(bcs.deserialize(b"\xff" * 8, st.int64), (-1, b""))

    def test_bcs_i128(self):
        self.assertEqual(
            bcs.serialize(st.int128(0x0102030405060708090A0B0C0D0E0F10), st.int128),
            b"\x10\x0f\x0e\r\x0c\x0b\n\t\x08\x07\x06\x05\x04\x03\x02\x01",
        )
        self.assertEqual(bcs.deserialize(b"\xff" * 16, st.int128), (st.int128(-1), b""))

    def test_encode_u32_as_uleb128(self):
        self.assertEqual(encode_u32_as_uleb128(0), b"\x00")
        self.assertEqual(encode_u32_as_uleb128(3), b"\x03")
        self.assertEqual(encode_u32_as_uleb128(0x7F), b"\x7f")
        self.assertEqual(encode_u32_as_uleb128(0x3F01), b"\x81\x7e")
        self.assertEqual(encode_u32_as_uleb128(0x8001), b"\x81\x80\x02")
        self.assertEqual(encode_u32_as_uleb128(bcs.MAX_U32), b"\xff\xff\xff\xff\x0f")

        self.assertEqual(decode_uleb128_as_u32(b"\x00"), 0)
        self.assertEqual(decode_uleb128_as_u32(b"\x03"), 3)
        self.assertEqual(decode_uleb128_as_u32(b"\x7f"), 0x7F)
        self.assertEqual(decode_uleb128_as_u32(b"\x81\x7e"), 0x3F01)
        self.assertEqual(decode_uleb128_as_u32(b"\x80\x80\x01"), 0x4000)
        self.assertEqual(decode_uleb128_as_u32(b"\x80\x80\x01"), 0x4000)
        self.assertEqual(decode_uleb128_as_u32(b"\x81\x80\x02"), 0x8001)
        self.assertEqual(decode_uleb128_as_u32(b"\xff\xff\xff\xff\x0f"), bcs.MAX_U32)
        with self.assertRaises(st.DeserializationError):
            decode_uleb128_as_u32(b"\x80\x00")
        with self.assertRaises(st.DeserializationError):
            decode_uleb128_as_u32(b"\xff\xff\xff\xff\x10")

    def test_encode_length(self):
        self.assertEqual(encode_length(bcs.MAX_LENGTH), b"\xff\xff\xff\xff\x07")
        with self.assertRaises(st.SerializationError):
            encode_length(bcs.MAX_LENGTH + 1)
        with self.assertRaises(st.DeserializationError):
            decode_length(b"\xff\xff\xff\xff\x08")

    def test_serialize_bytes(self):
        self.assertEqual(bcs.serialize(b"", bytes), b"\x00")
        self.assertEqual(bcs.serialize(b"\x00\x00", bytes), b"\x02\x00\x00")
        self.assertEqual(
            bcs.serialize(b"\x00" * 128, bytes), b"\x80\x01" + b"\x00" * 128
        )

        self.assertEqual(bcs.deserialize(b"\x00", bytes), (b"", b""))

    def test_serialize_vec_bytes(self):
        a = []
        b = [b"\x00\x00", b"\x01\x01"]
        c = [b"\x00" * 128, b"\x01" * 127, b"\x02" * 3, b""]

        aser = bcs.serialize(a, typing.Sequence[bytes])
        bser = bcs.serialize(b, typing.Sequence[bytes])
        cser = bcs.serialize(c, typing.Sequence[bytes])

        self.assertEqual(aser, b"\x00")
        self.assertEqual(bser, b"\x02\x02\x00\x00\x02\x01\x01")
        self.assertEqual(cser,
                b"\x04\x80\x01" + b"\x00" * 128 +
                b"\x7F" + b"\x01" * 127 +
                b"\x03" + b"\x02" * 3 +
                b"\x00"
        )

        self.assertEqual(bcs.deserialize(aser, typing.Sequence[bytes]), (a, b""))
        self.assertEqual(bcs.deserialize(bser, typing.Sequence[bytes]), (b, b""))
        self.assertEqual(bcs.deserialize(cser, typing.Sequence[bytes]), (c, b""))

    def test_serialize_tuple(self):
        T = typing.Tuple[st.uint8, st.uint16]
        self.assertEqual(bcs.serialize((0, 1), T), b"\x00\x01\x00")
        self.assertEqual(bcs.deserialize(b"\x02\x01\x00", T), ((2, 1), b""))

    def test_serialize_option(self):
        T = typing.Optional[st.uint16]
        self.assertEqual(bcs.serialize(None, T), b"\x00")
        self.assertEqual(bcs.serialize(6, T), b"\x01\x06\x00")
        self.assertEqual(bcs.deserialize(b"\x00", T), (None, b""))
        self.assertEqual(bcs.deserialize(b"\x01\x02\x00", T), (2, b""))
        with self.assertRaisesRegex(st.DeserializationError, "Wrong tag.*"):
            # Must enforce canonical encoding.
            bcs.deserialize(b"\x02\x06\x00", T)

    def test_serialize_sequence(self):
        Seq = typing.Sequence[st.uint16]
        self.assertEqual(bcs.serialize([], Seq), b"\x00")
        self.assertEqual(bcs.serialize([0, 1], Seq), b"\x02\x00\x00\x01\x00")
        self.assertEqual(
            bcs.serialize([256] * 128, Seq), b"\x80\x01" + b"\x00\x01" * 128
        )
        self.assertEqual(bcs.deserialize(b"\x01\x03\x00", Seq), ([3], b""))

    def test_serialize_str(self):
        self.assertEqual(bcs.serialize("ABC\u0394", str), b"\x05ABC\xce\x94")
        self.assertEqual(bcs.deserialize(b"\x05ABC\xce\x94A", str), ("ABC\u0394", b"A"))
        with self.assertRaises(st.DeserializationError):
            bcs.deserialize(b"\x03AB", str)
        with self.assertRaises(st.DeserializationError):
            bcs.deserialize(b"\x03\x80ab", str)

    def test_deserialize_long_sequence(self):
        Seq = typing.Sequence[st.uint16]
        five = st.uint16(5)
        v = [five] * 1000000
        b = bcs.serialize(v, Seq)
        self.assertEqual(bcs.deserialize(b, Seq), (v, b""))

    def test_serialize_map(self):
        Map = typing.Dict[st.uint16, st.uint8]
        m = OrderedDict([(1, 5), (256, 3)])
        e = bcs.serialize(m, Map)
        self.assertEqual(e, b"\x02\x00\x01\x03\x01\x00\x05")
        self.assertEqual(
            (m, b""), bcs.deserialize(b"\x02\x00\x01\x03\x01\x00\x05", Map)
        )
        m2 = OrderedDict([(256, 3), (1, 5)])
        e2 = bcs.serialize(m2, Map)
        self.assertEqual(e2, e)
        with self.assertRaises(st.DeserializationError):
            # Must enforce canonical encoding.
            bcs.deserialize(b"\x02\x01\x00\x05\x00\x01\x03", Map)

    def test_serialize_set(self):
        Set = typing.Dict[st.uint16, st.unit]
        m = {256: None, 1: None}
        e = bcs.serialize(m, Set)
        self.assertEqual(e, b"\x02\x00\x01\x01\x00")
        self.assertEqual((m, b""), bcs.deserialize(b"\x02\x00\x01\x01\x00", Set))
        with self.assertRaises(st.DeserializationError):
            # Must enforce canonical encoding.
            bcs.deserialize(b"\x02\x01\x00\x00\x01", Set)

    @dataclass
    class Foo:
        x: st.uint8
        y: st.uint16

    def test_struct(self):
        self.assertEqual(
            bcs.serialize(BcsTestCase.Foo(x=0, y=1), BcsTestCase.Foo), b"\x00\x01\x00"
        )
        self.assertEqual(
            bcs.deserialize(b"\x02\x01\x00", BcsTestCase.Foo),
            (BcsTestCase.Foo(x=2, y=1), b""),
        )

    class Bar:
        VARIANTS = []  # type: typing.Sequence[typing.Type['Bar']]

    @dataclass
    class Bar1(Bar):
        INDEX = 1
        x: st.uint8
        y: st.uint16

    Bar.VARIANTS = [None, Bar1, None]

    def test_enum(self):
        self.assertEqual(
            bcs.serialize(BcsTestCase.Bar1(x=0, y=1), BcsTestCase.Bar),
            b"\x01\x00\x01\x00",
        )
        self.assertEqual(
            bcs.deserialize(b"\x01\x02\x01\x00", BcsTestCase.Bar),
            (BcsTestCase.Bar1(x=2, y=1), b""),
        )

    @dataclass
    class List:
        next: typing.Optional[typing.Tuple[st.uint64, "BcsTestCase.List"]]

        @staticmethod
        def empty() -> "BcsTestCase.List":
            return BcsTestCase.List(next=None)

        @staticmethod
        def cons(value: st.uint64, tail: "BcsTestCase.List") -> "BcsTestCase.List":
            return BcsTestCase.List(next=(value, tail))

        @staticmethod
        def integers(size: int) -> "BcsTestCase.List":
            result = BcsTestCase.List.empty()
            for i in range(size):
                result = BcsTestCase.List.cons(st.uint64(i), result)
            return result

    def test_max_container_depth(self):
        # Required to avoid RecursionError's in python.
        sys.setrecursionlimit(bcs.MAX_CONTAINER_DEPTH * 5)

        l1 = BcsTestCase.List.integers(4)
        b1 = bcs.serialize(l1, BcsTestCase.List)
        self.assertEqual(
            b1,
            bytes(
                [
                    # fmt: off
                    1, 3, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ]
            ),
        )
        self.assertEqual(bcs.deserialize(b1, BcsTestCase.List), (l1, b""))

        l2 = BcsTestCase.List.integers(bcs.MAX_CONTAINER_DEPTH - 1)
        b2 = bcs.serialize(l2, BcsTestCase.List)
        self.assertEqual(bcs.deserialize(b2, BcsTestCase.List), (l2, b""))

        l3 = BcsTestCase.List.integers(bcs.MAX_CONTAINER_DEPTH)
        with self.assertRaises(st.SerializationError):
            bcs.serialize(l3, BcsTestCase.List)

        b3 = bytes([1, 243, 1, 0, 0, 0, 0, 0, 0]) + b2
        with self.assertRaisesRegex(
            st.DeserializationError, "Exceeded maximum container depth.*"
        ):
            self.assertEqual(bcs.deserialize(b3, BcsTestCase.List))

        # Pairs don't count in "container depth".
        P = typing.Tuple[BcsTestCase.List, BcsTestCase.List]
        self.assertEqual(bcs.deserialize(b2 + b2, P), ((l2, l2), b""))
        with self.assertRaisesRegex(
            st.DeserializationError, "Exceeded maximum container depth.*"
        ):
            bcs.deserialize(b2 + b3, P)
