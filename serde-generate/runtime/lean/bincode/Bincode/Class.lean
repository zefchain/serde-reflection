/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bincode.Basic

/-!
Type classes for Bincode serialization.

Instances are provided for primitive types and for `Option`, `Array` (sequences),
`Vector` (fixed-size arrays), pairs, and `Std.HashMap` (maps). Maps can also be represented by
arrays of entries, using `Bincode.Ser.map` and `Bincode.De.map`.
-/

namespace Bincode

open Serde

class Serialize (α : Type) where
  serialize : α → Ser Unit

/--
Deserializers. Only types that have values can be deserialized: the field `nonempty` is found
by instance inference (e.g. from an `Inhabited` instance), so there is no instance for types
without values such as `structure S where s : S`.
-/
class Deserialize (α : Type) where
  deserialize : De α
  nonempty : Nonempty α := by infer_instance

export Serialize (serialize)
export Deserialize (deserialize)

/-- Serializes a value into Bincode bytes. -/
def toBytes [Serialize α] (x : α) : Except Error ByteArray := Ser.run (serialize x)

/-- Deserializes a value from Bincode bytes. All the input must be consumed. -/
def fromBytes [Deserialize α] (input : ByteArray) : Except Error α := De.run deserialize input

instance : Serialize Unit := ⟨Ser.unit⟩
instance : Serialize Bool := ⟨Ser.bool⟩
instance : Serialize UInt8 := ⟨Ser.u8⟩
instance : Serialize UInt16 := ⟨Ser.u16⟩
instance : Serialize UInt32 := ⟨Ser.u32⟩
instance : Serialize UInt64 := ⟨Ser.u64⟩
instance : Serialize UInt128 := ⟨Ser.u128⟩
instance : Serialize Int8 := ⟨Ser.i8⟩
instance : Serialize Int16 := ⟨Ser.i16⟩
instance : Serialize Int32 := ⟨Ser.i32⟩
instance : Serialize Int64 := ⟨Ser.i64⟩
instance : Serialize Int128 := ⟨Ser.i128⟩
instance : Serialize Float32 := ⟨Ser.f32⟩
instance : Serialize Float := ⟨Ser.f64⟩
instance : Serialize Char := ⟨Ser.char⟩
instance : Serialize String := ⟨Ser.str⟩
instance : Serialize ByteArray := ⟨Ser.bytes⟩
instance [Serialize α] : Serialize (Option α) := ⟨Ser.option serialize⟩
instance [Serialize α] : Serialize (Array α) := ⟨Ser.seq serialize⟩
instance [Serialize α] : Serialize (Vector α n) := ⟨Ser.vector serialize⟩
instance [Serialize α] [Serialize β] : Serialize (α × β) := ⟨Ser.prod serialize serialize⟩
instance [BEq K] [Hashable K] [Serialize K] [Serialize V] : Serialize (Std.HashMap K V) :=
  ⟨Ser.hashMap serialize serialize⟩

instance : Deserialize Unit where deserialize := De.unit
instance : Deserialize Bool where deserialize := De.bool
instance : Deserialize UInt8 where deserialize := De.u8
instance : Deserialize UInt16 where deserialize := De.u16
instance : Deserialize UInt32 where deserialize := De.u32
instance : Deserialize UInt64 where deserialize := De.u64
instance : Deserialize UInt128 where deserialize := De.u128
instance : Deserialize Int8 where deserialize := De.i8
instance : Deserialize Int16 where deserialize := De.i16
instance : Deserialize Int32 where deserialize := De.i32
instance : Deserialize Int64 where deserialize := De.i64
instance : Deserialize Int128 where deserialize := De.i128
instance : Deserialize Float32 where deserialize := De.f32
instance : Deserialize Float where deserialize := De.f64
instance : Deserialize Char where deserialize := De.char
instance : Deserialize String where deserialize := De.str
instance : Deserialize ByteArray where deserialize := De.bytes
instance [Deserialize α] : Deserialize (Option α) where deserialize := De.option deserialize
instance [Deserialize α] : Deserialize (Array α) where deserialize := De.seq deserialize
instance [Deserialize α] : Deserialize (Vector α n) where
  deserialize := De.vector n deserialize
  nonempty := let ⟨a⟩ := Deserialize.nonempty (α := α); ⟨Vector.replicate n a⟩

instance [Deserialize α] [Deserialize β] : Deserialize (α × β) where
  deserialize := De.prod deserialize deserialize
  nonempty :=
    let ⟨a⟩ := Deserialize.nonempty (α := α)
    let ⟨b⟩ := Deserialize.nonempty (α := β)
    ⟨(a, b)⟩

instance [BEq K] [Hashable K] [Deserialize K] [Deserialize V] :
    Deserialize (Std.HashMap K V) where
  deserialize := De.hashMap deserialize deserialize

end Bincode
