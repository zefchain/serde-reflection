// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

part of 'bcs.dart';

class BcsSerializer extends BinarySerializer {
  BcsSerializer()
      : super(
          containerDepthBudget: maxContainerDepth,
        );

  void serializeUint32AsUleb128(int value) {
    while (((value & 0xFFFFFFFF) >> 7) != 0) {
      output.add((value & 0x7f) | 0x80);
      value = (value & 0xFFFFFFFF) >> 7;
    }
    output.add(value);
  }

  @override
  void serializeLength(int value) {
    serializeUint32AsUleb128(value);
  }

  @override
  void serializeVariantIndex(int value) {
    serializeUint32AsUleb128(value);
  }

  @override
  void sortMapEntries(List<int> offsets) {
    if (offsets.isEmpty) {
      return;
    }

    // Prepare a list of slices
    final data = Uint8List.fromList(output);
    List<Uint8List> slices = [];

    // Collect slices
    for (int i = 0; i < offsets.length; i++) {
      final int startOffset = offsets[i];
      final int cutOffset;
      if (i + 1 < offsets.length) {
        cutOffset = offsets[i + 1];
      } else {
        cutOffset = data.length;
      }
      slices.add(data.sublist(startOffset, cutOffset));
    }

    // Sort slices using lexicographic comparison
    slices.sort((a, b) {
      for (int i = 0; i < a.length && i < b.length; i++) {
        if (a[i] != b[i]) {
          return a[i].compareTo(b[i]);
        }
      }
      return a.length.compareTo(b.length);
    });

    // Write sorted slices back to output
    int writePosition = offsets[0];
    for (final slice in slices) {
      output.setRange(writePosition, writePosition + slice.length, slice);
      writePosition += slice.length;
    }

    // Ensure the final length is correct
    assert(offsets.last == output.length);
  }
}
