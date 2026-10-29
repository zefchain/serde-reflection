/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/

/-!
Lean has no built-in 128-bit integers. These types follow the conventions of the
fixed-width integer types of the core library (`UInt64`, `Int64`, ...): a value is a
wrapper around a `BitVec 128`, arithmetic is modular, and `Int128` uses two's complement.
-/

namespace Serde

/-- Unsigned 128-bit integers. -/
structure UInt128 where
  toBitVec : BitVec 128
  deriving DecidableEq, Inhabited, Hashable

/-- Signed 128-bit integers (two's complement). -/
structure Int128 where
  toBitVec : BitVec 128
  deriving DecidableEq, Inhabited, Hashable

namespace UInt128

def size : Nat := 2 ^ 128

def ofNat (n : Nat) : UInt128 := ⟨BitVec.ofNat 128 n⟩
def toNat (x : UInt128) : Nat := x.toBitVec.toNat

/-- Builds a value from its low and high 64-bit halves. -/
def ofHalves (lo hi : UInt64) : UInt128 := ofNat (lo.toNat + hi.toNat * 2 ^ 64)
def lo (x : UInt128) : UInt64 := x.toNat.toUInt64
def hi (x : UInt128) : UInt64 := (x.toNat / 2 ^ 64).toUInt64

instance : OfNat UInt128 n := ⟨ofNat n⟩
instance : Repr UInt128 := ⟨fun x p => reprPrec x.toNat p⟩
instance : ToString UInt128 := ⟨fun x => toString x.toNat⟩
instance : Ord UInt128 := ⟨fun x y => compare x.toNat y.toNat⟩
instance : LT UInt128 := ⟨fun x y => x.toNat < y.toNat⟩
instance : LE UInt128 := ⟨fun x y => x.toNat ≤ y.toNat⟩
instance (x y : UInt128) : Decidable (x < y) := inferInstanceAs (Decidable (x.toNat < y.toNat))
instance (x y : UInt128) : Decidable (x ≤ y) := inferInstanceAs (Decidable (x.toNat ≤ y.toNat))
instance : Add UInt128 := ⟨fun x y => ⟨x.toBitVec + y.toBitVec⟩⟩
instance : Sub UInt128 := ⟨fun x y => ⟨x.toBitVec - y.toBitVec⟩⟩
instance : Mul UInt128 := ⟨fun x y => ⟨x.toBitVec * y.toBitVec⟩⟩

theorem toNat_lt (x : UInt128) : x.toNat < 2 ^ 128 := x.toBitVec.isLt

theorem toNat_ofNat (n : Nat) : (ofNat n).toNat = n % 2 ^ 128 := by
  simp [toNat, ofNat]

theorem toNat_inj {x y : UInt128} : x.toNat = y.toNat ↔ x = y := by
  cases x; cases y
  simp [toNat, BitVec.toNat_inj]

theorem lo_ofHalves (lo hi : UInt64) : (ofHalves lo hi).lo = lo := by
  have := lo.toNat_lt; have := hi.toNat_lt
  apply UInt64.toNat_inj.mp
  simp [ofHalves, UInt128.lo, toNat_ofNat] <;> omega

theorem hi_ofHalves (lo hi : UInt64) : (ofHalves lo hi).hi = hi := by
  have := lo.toNat_lt; have := hi.toNat_lt
  apply UInt64.toNat_inj.mp
  simp [ofHalves, UInt128.hi, toNat_ofNat] <;> omega

theorem ofHalves_lo_hi (x : UInt128) : ofHalves x.lo x.hi = x := by
  have := x.toNat_lt
  apply toNat_inj.mp
  simp [ofHalves, UInt128.lo, UInt128.hi, toNat_ofNat] <;> omega

end UInt128

namespace Int128

def ofInt (i : Int) : Int128 := ⟨BitVec.ofInt 128 i⟩
def toInt (x : Int128) : Int := x.toBitVec.toInt
def ofNat (n : Nat) : Int128 := ⟨BitVec.ofNat 128 n⟩

/-- Reinterprets the bits of an unsigned value. -/
def ofUInt128 (x : UInt128) : Int128 := ⟨x.toBitVec⟩
/-- Reinterprets the bits as an unsigned value. -/
def toUInt128 (x : Int128) : UInt128 := ⟨x.toBitVec⟩

def minValue : Int128 := ofInt (-(2 ^ 127))
def maxValue : Int128 := ofInt (2 ^ 127 - 1)

instance : OfNat Int128 n := ⟨ofNat n⟩
instance : Neg Int128 := ⟨fun x => ⟨-x.toBitVec⟩⟩
instance : IntCast Int128 := ⟨ofInt⟩
instance : Repr Int128 := ⟨fun x p => reprPrec x.toInt p⟩
instance : ToString Int128 := ⟨fun x => toString x.toInt⟩
instance : Ord Int128 := ⟨fun x y => compare x.toInt y.toInt⟩
instance : LT Int128 := ⟨fun x y => x.toInt < y.toInt⟩
instance : LE Int128 := ⟨fun x y => x.toInt ≤ y.toInt⟩
instance (x y : Int128) : Decidable (x < y) := inferInstanceAs (Decidable (x.toInt < y.toInt))
instance (x y : Int128) : Decidable (x ≤ y) := inferInstanceAs (Decidable (x.toInt ≤ y.toInt))
instance : Add Int128 := ⟨fun x y => ⟨x.toBitVec + y.toBitVec⟩⟩
instance : Sub Int128 := ⟨fun x y => ⟨x.toBitVec - y.toBitVec⟩⟩
instance : Mul Int128 := ⟨fun x y => ⟨x.toBitVec * y.toBitVec⟩⟩

end Int128

end Serde
