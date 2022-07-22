// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package bcs_test

import (
	"fmt"
	"testing"

	"github.com/zefchain/serde-reflection/serde-generate/runtime/golang/bcs"
	"github.com/zefchain/serde-reflection/serde-generate/runtime/golang/serde"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestSerializeDeserializeBytes(t *testing.T) {
	cases := []struct {
		target   []byte
		expected []byte
	}{
		{
			target:   []byte{1, 2, 38},
			expected: []byte{3, 1, 2, 38},
		},
		{
			target:   []byte{},
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeBytes(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeBytes()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeBytes()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeStr(t *testing.T) {
	cases := []struct {
		target   string
		expected []byte
	}{
		{
			target:   "hello world!",
			expected: []byte{12, 104, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33},
		},
		{
			target:   "",
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(tc.target, func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeStr(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeStr()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeStr()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeBool(t *testing.T) {
	cases := []struct {
		target   bool
		expected []byte
	}{
		{
			target:   true,
			expected: []byte{1},
		},
		{
			target:   false,
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeBool(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeBool()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}

	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeBool()
		require.EqualError(t, err, "EOF")
	})

	t.Run("deserialize error: invalid byte", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{2})
		_, err := d.DeserializeBool()
		require.EqualError(t, err, "invalid bool byte: expected 0 / 1, but got 2")
	})
}

func TestSerializeDeserializeUnit(t *testing.T) {
	cases := []struct {
		target   struct{}
		expected []byte
	}{
		{
			target:   struct{}{},
			expected: []byte(nil),
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeUnit(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeUnit()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
}

func TestSerializeDeserializeU8(t *testing.T) {
	cases := []struct {
		target   uint8
		expected []byte
	}{
		{
			target:   ^uint8(0),
			expected: []byte{^uint8(0)},
		},
		{
			target:   0,
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeU8(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeU8()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}

	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeU8()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeU16(t *testing.T) {
	cases := []struct {
		target   uint16
		expected []byte
	}{
		{
			target:   ^uint16(0),
			expected: []byte{^uint8(0), ^uint8(0)},
		},
		{
			target:   0,
			expected: []byte{0, 0},
		},
		{
			target:   827,
			expected: []byte{59, 3},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeU16(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeU16()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeU16()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeU32(t *testing.T) {
	cases := []struct {
		target   uint32
		expected []byte
	}{
		{
			target:   ^uint32(0),
			expected: []byte{^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0)},
		},
		{
			target:   0,
			expected: []byte{0, 0, 0, 0},
		},
		{
			target:   827,
			expected: []byte{59, 3, 0, 0},
		},
		{
			target:   321243314,
			expected: []byte{178, 200, 37, 19},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeU32(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeU32()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeU32()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeU64(t *testing.T) {
	cases := []struct {
		target   uint64
		expected []byte
	}{
		{
			target:   ^uint64(0),
			expected: []byte{^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0)},
		},
		{
			target:   0,
			expected: []byte{0, 0, 0, 0, 0, 0, 0, 0},
		},
		{
			target:   827,
			expected: []byte{59, 3, 0, 0, 0, 0, 0, 0},
		},
		{
			target:   321243314,
			expected: []byte{178, 200, 37, 19, 0, 0, 0, 0},
		},
		{
			target:   2212444144212422242,
			expected: []byte{98, 174, 44, 37, 58, 46, 180, 30},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeU64(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeU64()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeU64()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeU128(t *testing.T) {
	cases := []struct {
		target   serde.Uint128
		expected []byte
	}{
		{
			target: serde.Uint128{
				^uint64(0),
				^uint64(0),
			},
			expected: []byte{
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
			},
		},
		{
			target: serde.Uint128{0, 0},
			expected: []byte{
				0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0,
			},
		},
		{
			target: serde.Uint128{High: 0, Low: 321243314},
			expected: []byte{
				178, 200, 37, 19, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0,
			},
		},
		{
			target: serde.Uint128{High: 321243314, Low: 827},
			expected: []byte{
				59, 3, 0, 0, 0, 0, 0, 0,
				178, 200, 37, 19, 0, 0, 0, 0,
			},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeU128(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeU128()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeU128()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeI8(t *testing.T) {
	cases := []struct {
		target   int8
		expected []byte
	}{
		{
			target:   ^int8(0),
			expected: []byte{^uint8(0)},
		},
		{
			target:   -^int8(0) - 1,
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeI8(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeI8()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeI8()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeI16(t *testing.T) {
	cases := []struct {
		target   int16
		expected []byte
	}{
		{
			target:   ^int16(0),
			expected: []byte{^uint8(0), ^uint8(0)},
		},
		{
			target:   0,
			expected: []byte{0, 0},
		},
		{
			target:   -2,
			expected: []byte{254, 255},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeI16(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeI16()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeI16()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeI32(t *testing.T) {
	cases := []struct {
		target   int32
		expected []byte
	}{
		{
			target:   ^int32(0),
			expected: []byte{255, 255, 255, 255},
		},
		{
			target:   0,
			expected: []byte{0, 0, 0, 0},
		},
		{
			target:   -232,
			expected: []byte{24, 255, 255, 255},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeI32(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeI32()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeI32()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeI64(t *testing.T) {
	cases := []struct {
		target   int64
		expected []byte
	}{
		{
			target: ^int64(0),
			expected: []byte{
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
			},
		},
		{
			target:   0,
			expected: []byte{0, 0, 0, 0, 0, 0, 0, 0},
		},
		{
			target:   -232,
			expected: []byte{24, 255, 255, 255, 255, 255, 255, 255},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeI64(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeI64()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeI64()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeI128(t *testing.T) {
	cases := []struct {
		target   serde.Int128
		expected []byte
	}{
		{
			target: serde.Int128{^int64(0), ^uint64(0)},
			expected: []byte{
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
				^uint8(0), ^uint8(0), ^uint8(0), ^uint8(0),
			},
		},
		{
			target: serde.Int128{0, 0},
			expected: []byte{
				0, 0, 0, 0, 0, 0, 0, 0,
				0, 0, 0, 0, 0, 0, 0, 0,
			},
		},
		{
			target: serde.Int128{High: -232, Low: 321243314},
			expected: []byte{
				178, 200, 37, 19, 0, 0, 0, 0,
				24, 255, 255, 255, 255, 255, 255, 255,
			},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeI128(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeI128()
			require.NoError(t, err)

			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeI128()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeVariantIndex(t *testing.T) {
	cases := []struct {
		target   uint32
		expected []byte
	}{
		{
			target:   9487,
			expected: []byte{143, 74},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeVariantIndex(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeVariantIndex()
			require.NoError(t, err)
			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeVariantIndex()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeLenLimit(t *testing.T) {
	t.Run("SerializeLen: length is too large", func(t *testing.T) {
		s := bcs.NewSerializer()
		err := s.SerializeLen(^uint64(0))
		assert.Error(t, err)
		assert.Equal(t, "length is too large", err.Error())
	})
	t.Run("DeserializeLen: length is too large", func(t *testing.T) {
		s := bcs.NewSerializer()
		err := s.SerializeVariantIndex(^uint32(0))
		assert.NoError(t, err)

		d := bcs.NewDeserializer(s.GetBytes())
		ret, err := d.DeserializeLen()
		assert.Equal(t, uint64(0), ret)
		require.Error(t, err)
		assert.Equal(t, "length is too large", err.Error())
	})

	t.Run("overflow while parsing uleb128-encoded uint32", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{255, 255, 255, 255, 255, 255, 255, 255})
		_, err := d.DeserializeLen()
		require.Error(t, err)
		assert.Equal(t, "overflow while parsing uleb128-encoded uint32 value", err.Error())
	})

	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeLen()
		require.EqualError(t, err, "EOF")
	})
}

func TestSerializeDeserializeOptionTag(t *testing.T) {
	cases := []struct {
		target   bool
		expected []byte
	}{
		{
			target:   true,
			expected: []byte{1},
		},
		{
			target:   false,
			expected: []byte{0},
		},
	}

	for _, tc := range cases {
		t.Run(fmt.Sprintf("%#v", tc.target), func(t *testing.T) {
			s := bcs.NewSerializer()
			d := bcs.NewDeserializer(tc.expected)

			err := s.SerializeOptionTag(tc.target)
			require.NoError(t, err)

			deserialized, err := d.DeserializeOptionTag()
			require.NoError(t, err)
			assert.Equal(t, tc.expected, s.GetBytes())
			assert.Equal(t, tc.target, deserialized)
		})
	}
	t.Run("deserialize error: EOF", func(t *testing.T) {
		d := bcs.NewDeserializer([]byte{})
		_, err := d.DeserializeOptionTag()
		require.EqualError(t, err, "EOF")
	})
}

func TestGetBufferOffset(t *testing.T) {
	s := bcs.NewSerializer()
	s.SerializeU64(0)
	assert.Equal(t, uint64(8), s.GetBufferOffset())

	d := bcs.NewDeserializer([]byte{0, 0, 0, 0, 0, 0, 0, 0})
	assert.Equal(t, uint64(0), d.GetBufferOffset())
	d.DeserializeU64()
	assert.Equal(t, uint64(8), d.GetBufferOffset())
}

func TestCheckThatKeySlicesAreIncreasing(t *testing.T) {
	d := bcs.NewDeserializer([]byte{0, 1, 2, 0, 2})
	// Offsets are taken from the input bytes.
	d.DeserializeU32()
	require.NoError(t, d.CheckThatKeySlicesAreIncreasing(serde.Slice{0, 3}, serde.Slice{3, 5}))
	require.Error(t, d.CheckThatKeySlicesAreIncreasing(serde.Slice{0, 3}, serde.Slice{0, 3}))
	require.Error(t, d.CheckThatKeySlicesAreIncreasing(serde.Slice{1, 3}, serde.Slice{3, 5}))
}

func TestSortMapEntries(t *testing.T) {
	s := bcs.NewSerializer()
	s.SerializeU8(255)
	s.SerializeU32(1)
	s.SerializeU32(1)
	s.SerializeU32(2)
	assert.Equal(t, s.GetBytes(), []byte{255 /**/, 1 /**/, 0, 0 /**/, 0, 1, 0 /**/, 0 /**/, 0 /**/, 2, 0, 0, 0})

	offsets := []uint64{1, 2, 4, 7, 8, 9}
	s.SortMapEntries(offsets)
	assert.Equal(t, s.GetBytes(), []byte{255 /**/, 0 /**/, 0 /**/, 0, 0 /**/, 0, 1, 0 /**/, 1 /**/, 2, 0, 0, 0})
}
