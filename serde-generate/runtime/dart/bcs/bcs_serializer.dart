// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

part of bcs;

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

  void sortMapEntries(Uint8List offsets) {
    if (offsets.isEmpty) {
      return;
    }

    final binOutput = Uint8List.fromList(output);

    // Create a list of slices based on offsets
    List<Uint8List> slices = [];
    int totalLength = binOutput.length;
    List<int> offsetList = List.from(offsets);
    offsetList.add(totalLength);

    for (int i = 1; i < offsetList.length; i++) {
      slices.add(binOutput.sublist(offsetList[i - 1], offsetList[i]));
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
    int writePosition = offsetList[0];
    for (final slice in slices) {
      output.setRange(writePosition, writePosition + slice.length, slice);
      writePosition += slice.length;
    }

    // Ensure the final length is correct
    assert(offsetList.last == output.length);
  }
}
