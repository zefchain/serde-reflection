/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde
import Std.Data.HashMap

/-!
# Binary Canonical Serialization (BCS)

Serialization and deserialization primitives for the
[BCS format](https://github.com/zefchain/bcs).

`Bcs.Ser` and `Bcs.De` are the monads `Serde.Ser` and `Serde.De`, shared by all formats.
The encodings that BCS shares with other formats (fixed-width integers, options, pairs, ...)
come from `Serde.Basic` and are re-exported here, e.g. as `Bcs.Ser.u32`. This file defines
the encodings specific to BCS: ULEB128 lengths and variant indices, maps sorted by the
encodings of their keys, and the limit on the depth of containers. Compound values are handled
by combinators that take the (de)serializer of their components explicitly, e.g.
`Bcs.Ser.seq Bcs.Ser.u32`.

The definitions are written with explicit recursion rather than loops so that their
correctness can be proved (see `Bcs.Lawful`).
-/

namespace Bcs

open Serde

/-- Maximum length of sequences, strings, byte arrays, and maps. -/
def maxSequenceLength : Nat := 2 ^ 31 - 1

/-- Maximum number of nested containers (structs and enums). -/
def maxContainerDepth : Nat := 500

/-- Serialization monad (see `Serde.Ser`). -/
abbrev Ser := Serde.Ser

/-- Deserialization monad (see `Serde.De`). -/
abbrev De := Serde.De

/-- Order of map keys: lexicographic order of their encodings. -/
@[inline] def compareKeys (a b : ByteArray) : Ordering := compare a.data b.data

namespace Ser

export Serde.Ser (modify capture withDepth container u8 u16 u32 u64 u128 i8 i16 i32 i64 i128
  bool unit raw option prod elems tupleArray vector)

/-- Writes a natural number in ULEB128 format. -/
def ulebNat (n : Nat) : Ser Unit :=
  if n < 0x80 then u8 n.toUInt8
  else do
    u8 (n % 0x80 + 0x80).toUInt8
    ulebNat (n / 0x80)
termination_by n
decreasing_by omega

/-- Writes a 32-bit integer in ULEB128 format. -/
@[inline] def uleb128 (x : UInt32) : Ser Unit := ulebNat x.toNat

/-- Writes the length of a sequence. -/
def len (n : Nat) : Ser Unit :=
  if n ≤ maxSequenceLength then ulebNat n else throw (.invalidLength n)

@[inline] def bytes (b : ByteArray) : Ser Unit := Serde.Ser.bytes len b

@[inline] def str (s : String) : Ser Unit := Serde.Ser.str len s

def char (_ : Char) : Ser Unit := throw (.unsupported "char")
def f32 (_ : Float32) : Ser Unit := throw (.unsupported "f32")
def f64 (_ : Float) : Ser Unit := throw (.unsupported "f64")

@[inline] def seq (f : α → Ser Unit) (xs : Array α) : Ser Unit := Serde.Ser.seq len f xs

/-- Serializes the key and the value of a map entry separately. -/
@[inline] def captureEntry (fk : K → Ser Unit) (fv : V → Ser Unit) (e : K × V) :
    Ser (ByteArray × ByteArray) := do
  let k ← capture (fk e.1)
  let v ← capture (fv e.2)
  return (k, v)

/--
Whether the keys of consecutive entries are strictly increasing, and greater than the
`previous` key, if any.
-/
def keysIncreasing (previous : Option ByteArray) : List (ByteArray × ByteArray) → Bool
  | [] => true
  | c :: rest => previous.all (compareKeys · c.1 == .lt) && keysIncreasing (some c.1) rest

/--
Writes map entries (without length), sorted by the encoding of their keys. Fails if two keys
have the same encoding.
-/
def entries (fk : K → Ser Unit) (fv : V → Ser Unit) (entries : Array (K × V)) : Ser Unit := do
  let chunks ← entries.toList.mapM (captureEntry fk fv)
  let sorted := chunks.mergeSort fun a b => (compareKeys a.1 b.1).isLE
  if keysIncreasing none sorted then
    sorted.forM fun c => do raw c.1; raw c.2
  else
    throw .invalidMapOrder

/--
Writes a map given as an array of entries. Entries are sorted by the encoding of their
keys, as required by BCS. Fails if two keys have the same encoding.
-/
def map (fk : K → Ser Unit) (fv : V → Ser Unit) (xs : Array (K × V)) : Ser Unit := do
  len xs.size
  entries fk fv xs

/-- Writes an enum value: the index of its variant, in ULEB128 format, then its payload. -/
@[inline] def enum (index : α → UInt32) (payload : α → Ser Unit) (x : α) : Ser Unit :=
  Serde.Ser.enum uleb128 index payload x

/-- Writes a hash map. Entries are sorted by the encodings of their keys. -/
@[inline] def hashMap [BEq K] [Hashable K] (fk : K → Ser Unit) (fv : V → Ser Unit)
    (m : Std.HashMap K V) : Ser Unit :=
  map fk fv m.toArray

/-- Runs a serializer. -/
def run (f : Ser Unit) : Except Error ByteArray := Serde.Ser.run f maxContainerDepth

end Ser

namespace De

export Serde.De (input position advance withDepth lazy container u8 u16 u32 u64 u128 i8 i16
  i32 i64 i128 unit bool raw option const prod initialCapacity elemsLoop elems tupleArray vector)

/--
Reads a natural number in ULEB128 format using at most `fuel` bytes. Redundant trailing
zeros are rejected, so that every number has a unique encoding.
-/
def ulebNat : (fuel : Nat) → De Nat
  | 0 => throw .invalidUleb128
  | fuel + 1 => do
    let byte ← u8
    if byte < 0x80 then
      return byte.toNat
    else
      let rest ← ulebNat fuel
      if rest = 0 then throw .invalidUleb128 else return byte.toNat - 0x80 + 0x80 * rest

/-- Reads a 32-bit integer in canonical ULEB128 format. -/
def uleb128 : De UInt32 := do
  let n ← ulebNat 5
  if n < 2 ^ 32 then return n.toUInt32 else throw .invalidUleb128

/-- Reads the length of a sequence. -/
def len : De Nat := do
  let n ← ulebNat 5
  if n ≤ maxSequenceLength then return n else throw (.invalidLength n)

@[inline] def bytes : De ByteArray := Serde.De.bytes len

@[inline] def str : De String := Serde.De.str len

def char : De Char := throw (.unsupported "char")
def f32 : De Float32 := throw (.unsupported "f32")
def f64 : De Float := throw (.unsupported "f64")

@[inline] def seq (f : De α) : De (Array α) := Serde.De.seq len f

/--
Reads `n` map entries and appends them to `acc`, checking that the encodings of the keys
are strictly increasing. `previous` is the encoding of the previous key, if any.
-/
@[specialize] def entries (fk : De K) (fv : De V) :
    (n : Nat) → (previous : Option ByteArray) → (acc : Array (K × V)) → De (Array (K × V))
  | 0, _, acc => pure acc
  | n + 1, previous, acc => do
    let start ← position
    let k ← fk
    let key := (← input).extract start (← position)
    if previous.all (compareKeys · key == .lt) then
      let v ← fv
      entries fk fv n (some key) (acc.push (k, v))
    else
      throw .invalidMapOrder

/-- Reads a map as an array of entries sorted by the encodings of their keys. -/
def map (fk : De K) (fv : De V) : De (Array (K × V)) := do
  let n ← len
  entries fk fv n none (Array.mkEmpty (initialCapacity n))

/-- Reads a hash map, checking that the encodings of the keys are strictly increasing. -/
@[inline] def hashMap [BEq K] [Hashable K] (fk : De K) (fv : De V) : De (Std.HashMap K V) :=
  (fun xs => Std.HashMap.ofList xs.toList) <$> map fk fv

/--
Reads an enum value: the index of its variant, in ULEB128 format, then the payload, using the
deserializer `payload i` of the variant of index `i`. Fails if `payload i` is `none`.
-/
@[inline] def enum (name : String) (payload : Nat → Option (De α)) : De α :=
  Serde.De.enum uleb128 name payload

/-- Runs a deserializer on the full input, failing if some bytes are left. -/
def run (f : De α) (input : ByteArray) : Except Error α :=
  Serde.De.run f maxContainerDepth input

end De

end Bcs
