// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

part of 'bcs.dart';

// Maximum length allowed for sequences (vectors, bytes, strings) and maps.
const maxSequenceLength = (1 << 31) - 1;

// Maximum number of nested structs and enum variants.
const maxContainerDepth = 500;

class BcsDeserializer extends BinaryDeserializer {
  BcsDeserializer(Uint8List input)
      : super(
          input: input,
          containerDepthBudget: maxContainerDepth,
        );

  int deserializeUleb128AsUint32() {
    var value = 0;
    for (var shift = 0; shift < 32; shift += 7) {
      final x = super.deserializeUint8();
      final digit = (x & 0x7F);
      value = value | (digit << shift);
      if (value > maxInt) {
        throw Exception('Overflow while parsing uleb128-encoded uint32 value');
      }
      if (digit == x) {
        if (shift > 0 && digit == 0) {
          throw Exception('Invalid uleb128 number (unexpected zero digit)');
        }
        return value;
      }
    }
    throw Exception('Overflow while parsing uleb128-encoded uint32 value');
  }

  @override
  int deserializeLength() {
    final length = deserializeUleb128AsUint32();
    if (length > maxSequenceLength) {
      throw Exception("length is too large");
    }
    return length;
  }

  @override
  int deserializeVariantIndex() {
    return deserializeUleb128AsUint32();
  }

  @override
  void checkThatKeySlicesAreIncreasing(Slice key1, Slice key2) {
    if (Slice.compareBytes(input.buffer.asUint8List(), key1, key2) >= 0) {
      throw Exception(
          "Error while decoding map: keys are not serialized in the expected order");
    }
  }
}
