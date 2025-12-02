// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

import 'dart:typed_data';

import 'package:test/test.dart';

import '../serde/serde.dart';

void main() {
  test('Uint64', () {
    expect(Uint64.parse('0').toString(), '0');
    expect(Uint64.parse('184').toString(), '184');
    expect(Uint64.parse('18446744073709551615').toString(),
        '18446744073709551615');
  });

  test('Uint128', () {
    expect(Uint128.parse('0').toString(), '0');
    expect(Uint128.parse('340').toString(), '340');
    expect(Uint128.parse('340282366920938463463374607431768211455').toString(),
        '340282366920938463463374607431768211455');
  });

  test('Int128', () {
    expect(Int128.parse('-170141183460469231731687303715884105728').toString(),
        '-170141183460469231731687303715884105728');
    expect(Int128.parse('-170').toString(), '-170');
    expect(Int128.parse('170').toString(), '170');
    expect(Int128.parse('170141183460469231731687303715884105727').toString(),
        '170141183460469231731687303715884105727');
  });

  test('Bytes', () {
    final a = Bytes(Uint8List.fromList([0]));
    final b = Bytes(Uint8List.fromList([0]));
    final c = Bytes(Uint8List.fromList([1]));

    expect(a, b, reason: 'a == b');
    expect(a.hashCode, b.hashCode, reason: 'a and b have same hashCode');

    expect(a, isNot(c), reason: 'a != c');
    expect(a.hashCode, isNot(c.hashCode), reason: 'a and c have different hashCode');
  });
}
