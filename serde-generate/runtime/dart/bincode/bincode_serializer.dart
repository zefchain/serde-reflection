// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

part of bincode;

class BincodeSerializer extends BinarySerializer {
  @override
  void serializeLength(int value) {
    serializeUint64(Uint64(BigInt.from(value)));
  }

  @override
  void serializeVariantIndex(int value) {
    serializeUint32(value);
  }

  void sortMapEntries(Int32List offsets) {
    // Not required by the format.
  }
}
