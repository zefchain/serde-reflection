// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

part of 'bincode.dart';

class BincodeSerializer extends BinarySerializer {
  BincodeSerializer()
      : super(
          containerDepthBudget: maxContainerDepth,
        );

  @override
  void serializeLength(int len) {
    serializeUint64(Uint64(BigInt.from(len)));
  }

  @override
  void serializeVariantIndex(int index) {
    serializeUint32(index);
  }

  @override
  void sortMapEntries(List<int> offsets) {
    // Not required by the format.
  }
}
