/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Bytes
import Serde.Monad

/-!
# Shared primitives and combinators

Encodings that are common to the binary formats: little-endian fixed-width integers, booleans
and options as one-byte tags, pairs and fixed-size arrays as the concatenation of their
components. Sequences, byte arrays, strings, and enums are parameterized by the encodings of
lengths and of variant indices, which differ between formats.

Compound values are handled by combinators that take the (de)serializer of their components
explicitly, e.g. `Serde.Ser.option Serde.Ser.u32`. The definitions are written with explicit
recursion rather than loops so that their correctness can be proved (see `Serde.Lawful`).
-/

namespace Serde

namespace Ser

@[inline] def u8 (x : UInt8) : Ser Unit := modify (·.push x)
@[inline] def u16 (x : UInt16) : Ser Unit := modify (Bytes.pushUInt16LE · x)
@[inline] def u32 (x : UInt32) : Ser Unit := modify (Bytes.pushUInt32LE · x)
@[inline] def u64 (x : UInt64) : Ser Unit := modify (Bytes.pushUInt64LE · x)
@[inline] def u128 (x : UInt128) : Ser Unit := modify (Bytes.pushUInt128LE · x)
@[inline] def i8 (x : Int8) : Ser Unit := u8 x.toUInt8
@[inline] def i16 (x : Int16) : Ser Unit := u16 x.toUInt16
@[inline] def i32 (x : Int32) : Ser Unit := u32 x.toUInt32
@[inline] def i64 (x : Int64) : Ser Unit := u64 x.toUInt64
@[inline] def i128 (x : Int128) : Ser Unit := u128 x.toUInt128
@[inline] def bool (x : Bool) : Ser Unit := u8 (if x then 1 else 0)
@[inline] def unit (_ : Unit) : Ser Unit := pure ()

/-- Writes raw bytes, without length. -/
@[inline] def raw (b : ByteArray) : Ser Unit := modify (· ++ b)

/-- Writes bytes prefixed by their length, written with `len`. -/
@[inline] def bytes (len : Nat → Ser Unit) (b : ByteArray) : Ser Unit := do
  len b.size
  raw b

/-- Writes a string as its UTF-8 bytes, prefixed by their length, written with `len`. -/
@[inline] def str (len : Nat → Ser Unit) (s : String) : Ser Unit := bytes len s.toUTF8

@[inline] def option (f : α → Ser Unit) : Option α → Ser Unit
  | none => u8 0
  | some x => do u8 1; f x

/-- Writes a pair. -/
@[inline] def prod (f : α → Ser Unit) (g : β → Ser Unit) (p : α × β) : Ser Unit := do
  f p.1
  g p.2

/-- Writes the elements `xs[i:]` (loop of `Ser.elems`). -/
@[specialize] def elemsFrom (f : α → Ser Unit) (xs : Array α) (d i : Nat) (out : ByteArray) :
    EStateM.Result Error ByteArray Unit :=
  if h : i < xs.size then
    match f xs[i] d out with
    | .ok _ out => elemsFrom f xs d (i + 1) out
    | .error e out => .error e out
  else .ok () out
termination_by xs.size - i

/--
Writes the elements of an array (no length). This is `xs.forM f`, written as a loop that the
compiler can specialize.
-/
@[inline] def elems (f : α → Ser Unit) (xs : Array α) : Ser Unit :=
  ⟨fun d out => elemsFrom f xs d 0 out⟩

/-- Writes a sequence: its length, written with `len`, then its elements. -/
@[inline] def seq (len : Nat → Ser Unit) (f : α → Ser Unit) (xs : Array α) : Ser Unit := do
  len xs.size
  elems f xs

/-- Writes the elements of a fixed-size array (no length), checking its size. -/
@[inline] def tupleArray (size : Nat) (f : α → Ser Unit) (xs : Array α) : Ser Unit :=
  if xs.size = size then elems f xs else throw (.invalidLength xs.size)

@[inline] def vector (f : α → Ser Unit) (xs : Vector α n) : Ser Unit :=
  elems f xs.toArray

/--
Writes an enum value: the index of its variant, written with `variantIndex`, then its
payload.
-/
@[inline] def enum (variantIndex : UInt32 → Ser Unit) (index : α → UInt32)
    (payload : α → Ser Unit) (x : α) : Ser Unit := do
  variantIndex (index x)
  payload x

end Ser

namespace De

@[inline] def u8 : De UInt8 := do
  let p ← advance 1
  return (← input).get! p

@[inline] def u16 : De UInt16 := do
  let p ← advance 2
  return Bytes.getUInt16LE (← input) p

@[inline] def u32 : De UInt32 := do
  let p ← advance 4
  return Bytes.getUInt32LE (← input) p

@[inline] def u64 : De UInt64 := do
  let p ← advance 8
  return Bytes.getUInt64LE (← input) p

@[inline] def u128 : De UInt128 := do
  let p ← advance 16
  return Bytes.getUInt128LE (← input) p

@[inline] def i8 : De Int8 := UInt8.toInt8 <$> u8
@[inline] def i16 : De Int16 := UInt16.toInt16 <$> u16
@[inline] def i32 : De Int32 := UInt32.toInt32 <$> u32
@[inline] def i64 : De Int64 := UInt64.toInt64 <$> u64
@[inline] def i128 : De Int128 := Int128.ofUInt128 <$> u128
@[inline] def unit : De Unit := pure ()

/-
`bool`, `option`, and `str` branch on the result of a deserializer explicitly rather than in a
`do` block, so that the compiler does not build a deserializer for each branch at run time.
-/

def bool : De Bool := ⟨fun ctx pos =>
  match u8 ctx pos with
  | .ok b pos =>
    if b = 0 then .ok false pos else if b = 1 then .ok true pos else .error (.invalidBool b) pos
  | .error e pos => .error e pos⟩

/-- Reads `n` raw bytes. -/
def raw (n : Nat) : De ByteArray := do
  let p ← advance n
  return (← input).extract p (p + n)

/-- Reads bytes prefixed by their length, read with `len`. -/
@[inline] def bytes (len : De Nat) : De ByteArray := do raw (← len)

/-- Reads a string as UTF-8 bytes prefixed by their length, read with `len`. -/
@[inline] def str (len : De Nat) : De String := ⟨fun ctx pos =>
  match bytes len ctx pos with
  | .ok b pos =>
    match String.fromUTF8? b with
    | some s => .ok s pos
    | none => .error .invalidUtf8 pos
  | .error e pos => .error e pos⟩

@[inline] def option (f : De α) : De (Option α) := ⟨fun ctx pos =>
  match u8 ctx pos with
  | .ok b pos =>
    if b = 0 then .ok none pos
    else if b = 1 then (some <$> f) ctx pos
    else .error (.invalidOptionTag b) pos
  | .error e pos => .error e pos⟩

/-- Reads nothing and returns `c`. -/
@[inline] def const (c : α) : De α := (fun _ => c) <$> unit

/-- Reads a pair. -/
@[inline] def prod (f : De α) (g : De β) : De (α × β) := do
  let a ← f
  let b ← g
  return (a, b)

/-- Initial capacity for arrays whose length comes from untrusted input. -/
def initialCapacity (n : Nat) : Nat := min n 4096

/--
Reads `n` elements and appends them to `acc` (loop of `De.elems`). The deserializer of the
elements is passed as a function so that the compiler can specialize the loop.
-/
@[specialize] def elemsLoop (f : DeContext → Nat → EStateM.Result Error Nat α)
    (ctx : DeContext) : (n : Nat) → (acc : Array α) → (pos : Nat) →
      EStateM.Result Error Nat (Array α)
  | 0, acc, pos => .ok acc pos
  | n + 1, acc, pos =>
    match f ctx pos with
    | .ok a pos => elemsLoop f ctx n (acc.push a) pos
    | .error e pos => .error e pos

/-- Reads `n` elements and appends them to `acc`. -/
@[inline] def elems (f : De α) (n : Nat) (acc : Array α) : De (Array α) :=
  ⟨fun ctx pos => elemsLoop f.toFun ctx n acc pos⟩

/-- Reads exactly `n` elements. -/
@[inline] def tupleArray (n : Nat) (f : De α) : De (Array α) :=
  elems f n (Array.mkEmpty (initialCapacity n))

/-- Reads a sequence: its length, read with `len`, then its elements. -/
@[inline] def seq (len : De Nat) (f : De α) : De (Array α) := do tupleArray (← len) f

@[inline] def vector (n : Nat) (f : De α) : De (Vector α n) := do
  let xs ← tupleArray n f
  if h : xs.size = n then return ⟨xs, h⟩ else throw (.invalidLength xs.size)

/--
Reads an enum value: the index `i` of its variant, read with `variantIndex`, then its payload,
using the deserializer `payload i`. Fails if `payload i` is `none`.
-/
@[inline] def enum (variantIndex : De UInt32) (name : String)
    (payload : Nat → Option (De α)) : De α := do
  let i ← variantIndex
  match payload i.toNat with
  | some f => f
  | none => throw (.unknownVariant name i.toNat)

end De

end Serde
