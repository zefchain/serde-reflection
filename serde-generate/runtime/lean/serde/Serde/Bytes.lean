/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Int128

/-!
Little-endian reading and writing of fixed-width integers in byte arrays.

Readers do not check bounds: callers must make sure that enough bytes are available. They use
`ByteArray.get!` rather than `b[i]!`, which compiles to more efficient code.
-/

instance : Repr ByteArray where
  reprPrec b _ := "ByteArray.mk " ++ repr b.data

namespace Serde.Bytes

@[inline] def pushUInt16LE (b : ByteArray) (x : UInt16) : ByteArray :=
  (b.push x.toUInt8).push (x >>> 8).toUInt8

@[inline] def pushUInt32LE (b : ByteArray) (x : UInt32) : ByteArray :=
  (((b.push x.toUInt8).push (x >>> 8).toUInt8).push (x >>> 16).toUInt8).push (x >>> 24).toUInt8

@[inline] def pushUInt64LE (b : ByteArray) (x : UInt64) : ByteArray :=
  pushUInt32LE (pushUInt32LE b x.toUInt32) (x >>> 32).toUInt32

@[inline] def pushUInt128LE (b : ByteArray) (x : UInt128) : ByteArray :=
  pushUInt64LE (pushUInt64LE b x.lo) x.hi

@[inline] def getUInt16LE (b : ByteArray) (pos : Nat) : UInt16 :=
  (b.get! pos).toUInt16 + (b.get! (pos + 1)).toUInt16 * 0x100

@[inline] def getUInt32LE (b : ByteArray) (pos : Nat) : UInt32 :=
  (b.get! pos).toUInt32 + (b.get! (pos + 1)).toUInt32 * 0x100 +
    (b.get! (pos + 2)).toUInt32 * 0x10000 + (b.get! (pos + 3)).toUInt32 * 0x1000000

@[inline] def getUInt64LE (b : ByteArray) (pos : Nat) : UInt64 :=
  (getUInt32LE b pos).toUInt64 + (getUInt32LE b (pos + 4)).toUInt64 * 0x100000000

@[inline] def getUInt128LE (b : ByteArray) (pos : Nat) : UInt128 :=
  UInt128.ofHalves (getUInt64LE b pos) (getUInt64LE b (pos + 8))

end Serde.Bytes
