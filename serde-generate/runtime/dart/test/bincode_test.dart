// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

import 'dart:typed_data';
import 'package:test/test.dart';
import '../bincode/bincode.dart';

void main() {
  test('serializeUint32', () {
    BincodeSerializer serializer = BincodeSerializer();
    serializer.serializeUint32(1);
    expect(serializer.bytes, Uint8List.fromList([1, 0, 0, 0]));
  });

  test('deserializeUint32', () {
    BincodeDeserializer serializer =
        BincodeDeserializer(Uint8List.fromList([1, 0, 0, 0]));
    int result = serializer.deserializeUint32();
    expect(result, 1);
  });

  test('slice', () {
    BincodeSerializer serializer = BincodeSerializer();
    serializer.serializeUint8(-1);
    serializer.serializeUint32(1);
    serializer.serializeUint32(1);
    serializer.serializeUint32(2);
    expect(
        serializer.bytes,
        Uint8List.fromList([
          -1,
          /**/ 1,
          /**/ 0,
          0,
          /**/ 0,
          1,
          0,
          /**/ 0,
          /**/ 0,
          /**/ 2,
          0,
          0,
          0
        ]));
  });

  test('serializeUint8', () {
    final serializer = BincodeSerializer();
    serializer.serializeUint8(255);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeUint8(), 255);
    expect(() => serializer.serializeUint8(256), throwsException);
  });

  test('serializeUint16', () {
    final serializer = BincodeSerializer();
    serializer.serializeUint16(65535);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeUint16(), 65535);
    expect(() => serializer.serializeUint16(65536), throwsException);
  });

  test('serializeUint32', () {
    final serializer = BincodeSerializer();
    serializer.serializeUint32(4294967295);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeUint32(), 4294967295);
    expect(() => serializer.serializeUint32(4294967296), throwsException);
  });

  test('serializeInt8', () {
    final serializer = BincodeSerializer();
    serializer.serializeInt8(127);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt8(), 127);
    expect(() => serializer.serializeInt8(128), throwsException);
    expect(() => serializer.serializeInt8(-129), throwsException);
  });

  test('serializeInt16', () {
    final serializer = BincodeSerializer();
    serializer.serializeInt16(32767);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt16(), 32767);
    expect(() => serializer.serializeInt16(32768), throwsException);
    expect(() => serializer.serializeInt16(-32769), throwsException);
  });

  test('serializeInt32', () {
    final serializer = BincodeSerializer();
    serializer.serializeInt32(2147483647);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt32(), 2147483647);
    expect(() => serializer.serializeInt32(2147483648), throwsException);
    expect(() => serializer.serializeInt32(-2147483649), throwsException);
  });

  test('serializeInt64', () {
    final serializer = BincodeSerializer();
    serializer.serializeInt64(1);
    expect(serializer.bytes,
        Uint8List.fromList([1, 0, 0, 0, 0, 0, 0, 0]));
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt64(), 1);
  });

  test('serializeInt64 negative', () {
    final serializer = BincodeSerializer();
    serializer.serializeInt64(-1);
    expect(serializer.bytes,
        Uint8List.fromList([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]));
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt64(), -1);
  });

  test('serializeInt64 max safe integer', () {
    // 2^53 - 1 (JS MAX_SAFE_INTEGER) — largest value portable across all platforms
    final serializer = BincodeSerializer();
    serializer.serializeInt64(9007199254740991);
    expect(serializer.bytes,
        Uint8List.fromList([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x1F, 0x00]));
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt64(), 9007199254740991);
  });

  test('serializeInt64 min safe integer', () {
    // -(2^53 - 1) (JS MIN_SAFE_INTEGER)
    final serializer = BincodeSerializer();
    serializer.serializeInt64(-9007199254740991);
    expect(serializer.bytes,
        Uint8List.fromList([0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0xFF]));
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt64(), -9007199254740991);
  });

  test('serializeInt64 typical timestamp', () {
    // A realistic epoch-millis value (2026-03-23T00:00:00Z)
    final serializer = BincodeSerializer();
    serializer.serializeInt64(1774243200000);
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeInt64(), 1774243200000);
  });

  test('serializeString', () {
    final serializer = BincodeSerializer();
    serializer.serializeString('dummy text / ダミーテキスト');
    final deserializer = BincodeDeserializer(serializer.bytes);
    expect(deserializer.deserializeString(), 'dummy text / ダミーテキスト');
  });
}
