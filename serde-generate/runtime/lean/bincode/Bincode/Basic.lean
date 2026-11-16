/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde
import Std.Data.HashMap

/-!
# Bincode

Serialization and deserialization primitives for the default configuration of
[Bincode 1.x](https://docs.rs/bincode/1.3.3/bincode/): little-endian fixed-width integers,
64-bit lengths, and 32-bit variant indices.

`Bincode.Ser` and `Bincode.De` are the monads `Serde.Ser` and `Serde.De`, shared by all
formats. The encodings that Bincode shares with other formats (fixed-width integers, options,
pairs, ...) come from `Serde.Basic` and are re-exported here, e.g. as `Bincode.Ser.u32`. This
file defines the encodings specific to Bincode. Compound values are handled by combinators
that take the (de)serializer of their components explicitly, e.g.
`Bincode.Ser.seq Bincode.Ser.u32`.
-/

namespace Bincode

open Serde

/--
Maximum length of sequences, strings, byte arrays, and maps. Bincode writes lengths as 64-bit
integers, but, as in the runtimes of the other languages, lengths are limited to the maximal
size of arrays in most languages, so that malformed inputs cannot announce huge sequences.
-/
def maxSequenceLength : Nat := 2 ^ 31 - 1

/--
Container-depth budget. Bincode does not limit the depth of containers: this budget is larger
than the depth of any value that fits in memory. It only makes the definitions of recursive
types total.
-/
def maxContainerDepth : Nat := 2 ^ 62

/-- Serialization monad (see `Serde.Ser`). -/
abbrev Ser := Serde.Ser

/-- Deserialization monad (see `Serde.De`). -/
abbrev De := Serde.De

namespace Ser

export Serde.Ser (modify capture withDepth container u8 u16 u32 u64 u128 i8 i16 i32 i64 i128
  bool unit raw option prod elems tupleArray vector)

@[inline] def f32 (x : Float32) : Ser Unit := u32 x.toBits
@[inline] def f64 (x : Float) : Ser Unit := u64 x.toBits

/-- Writes a character as its UTF-8 encoding. -/
@[inline] def char (c : Char) : Ser Unit := raw (String.utf8EncodeChar c).toByteArray

/-- Writes the length of a sequence, as a 64-bit integer. -/
@[inline] def len (n : Nat) : Ser Unit :=
  if n ≤ maxSequenceLength then u64 n.toUInt64 else throw (.invalidLength n)

@[inline] def bytes (b : ByteArray) : Ser Unit := Serde.Ser.bytes len b

@[inline] def str (s : String) : Ser Unit := Serde.Ser.str len s

@[inline] def seq (f : α → Ser Unit) (xs : Array α) : Ser Unit := Serde.Ser.seq len f xs

/-- Writes a map given as an array of entries, in the given order. -/
@[inline] def map (fk : K → Ser Unit) (fv : V → Ser Unit) (xs : Array (K × V)) : Ser Unit :=
  seq (prod fk fv) xs

/-- Writes a hash map, in the order of iteration of its entries. -/
@[inline] def hashMap [BEq K] [Hashable K] (fk : K → Ser Unit) (fv : V → Ser Unit)
    (m : Std.HashMap K V) : Ser Unit :=
  map fk fv m.toArray

/-- Writes an enum value: the index of its variant, as a 32-bit integer, then its payload. -/
@[inline] def enum (index : α → UInt32) (payload : α → Ser Unit) (x : α) : Ser Unit :=
  Serde.Ser.enum u32 index payload x

/-- Runs a serializer. -/
def run (f : Ser Unit) : Except Error ByteArray := Serde.Ser.run f maxContainerDepth

end Ser

namespace De

export Serde.De (input position advance withDepth lazy container u8 u16 u32 u64 u128 i8 i16
  i32 i64 i128 unit bool raw option const prod initialCapacity elemsLoop elems tupleArray vector)

@[inline] def f32 : De Float32 := Float32.ofBits <$> u32
@[inline] def f64 : De Float := Float.ofBits <$> u64

/-- Reads a character encoded in UTF-8. -/
@[inline] def char : De Char := ⟨fun ctx pos =>
  match ctx.input.utf8DecodeChar? pos with
  | some c => .ok c (pos + c.utf8Size)
  | none => .error .invalidChar pos⟩

/-- Reads the length of a sequence, as a 64-bit integer. -/
def len : De Nat := do
  let n ← u64
  if n.toNat ≤ maxSequenceLength then return n.toNat else throw (.invalidLength n.toNat)

@[inline] def bytes : De ByteArray := Serde.De.bytes len

@[inline] def str : De String := Serde.De.str len

@[inline] def seq (f : De α) : De (Array α) := Serde.De.seq len f

/-- Reads a map as an array of entries. -/
@[inline] def map (fk : De K) (fv : De V) : De (Array (K × V)) := seq (prod fk fv)

/-- Reads a hash map. If a key occurs several times, the last entry is kept. -/
@[inline] def hashMap [BEq K] [Hashable K] (fk : De K) (fv : De V) : De (Std.HashMap K V) :=
  (fun xs => Std.HashMap.ofList xs.toList) <$> map fk fv

/--
Reads an enum value: the index of its variant, as a 32-bit integer, then the payload, using
the deserializer `payload i` of the variant of index `i`. Fails if `payload i` is `none`.
-/
@[inline] def enum (name : String) (payload : Nat → Option (De α)) : De α :=
  Serde.De.enum u32 name payload

/-- Runs a deserializer on the full input, failing if some bytes are left. -/
def run (f : De α) (input : ByteArray) : Except Error α :=
  Serde.De.run f maxContainerDepth input

end De

end Bincode
