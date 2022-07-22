// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package bincode

import (
	"errors"
	"math"

	"github.com/aptos-labs/serde-reflection/serde-generate/runtime/golang/serde"
)

// MaxSequenceLength is max length supported in practice (e.g. in Java).
const MaxSequenceLength = (1 << 31) - 1

// `deserializer` extends `serde.BinaryDeserializer` to implement `serde.Deserializer`.
type deserializer struct {
	serde.BinaryDeserializer
}

func NewDeserializer(input []byte) serde.Deserializer {
	return &deserializer{*serde.NewBinaryDeserializer(input, math.MaxUint64)}
}

func (d *deserializer) DeserializeF32() (float32, error) {
	ret, err := d.DeserializeU32()
	return math.Float32frombits(ret), err
}

func (d *deserializer) DeserializeF64() (float64, error) {
	ret, err := d.DeserializeU64()
	return math.Float64frombits(ret), err
}

func (d *deserializer) DeserializeBytes() ([]byte, error) {
	return d.BinaryDeserializer.DeserializeBytes(d.DeserializeLen)
}

func (d *deserializer) DeserializeStr() (string, error) {
	return d.BinaryDeserializer.DeserializeStr(d.DeserializeLen)
}

func (d *deserializer) DeserializeLen() (uint64, error) {
	ret, err := d.DeserializeU64()
	if ret > MaxSequenceLength {
		return 0, errors.New("length is too large")
	}
	return uint64(ret), err
}

func (d *deserializer) DeserializeVariantIndex() (uint32, error) {
	return d.DeserializeU32()
}

func (d *deserializer) CheckThatKeySlicesAreIncreasing(key1, key2 serde.Slice) error {
	// No need to check key ordering in Bincode.
	return nil
}
