// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

package bincode

import (
	"github.com/novifinancial/serde-reflection/serde-generate/runtime/golang/serde"
)

// `serializer` extends `serde.BinarySerializer` to implement `serde.Serializer`.
type serializer struct {
	serde.BinarySerializer
}

func NewSerializer() serde.Serializer {
	return new(serializer)
}

func (s *serializer) SerializeStr(value string) error {
	return s.BinarySerializer.SerializeStr(value, s.SerializeLen)
}

func (s *serializer) SerializeBytes(value []byte) error {
	return s.BinarySerializer.SerializeBytes(value, s.SerializeLen)
}

func (s *serializer) SerializeLen(value uint64) error {
	return s.SerializeU64(value)
}

func (s *serializer) SerializeVariantIndex(value uint32) error {
	return s.SerializeU32(value)
}

func (s *serializer) SortMapEntries(offsets []uint64) {
	// No need to sort map entries in Bincode.
}