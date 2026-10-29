/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bincode.Class
import Bincode.Lawful.Map

/-!
# Correctness of `Bincode.toBytes` and `Bincode.fromBytes`

`LawfulSerialize α` states that the `Serialize` and `Deserialize` instances of `α` are
correct, and `InjectiveSerialize α` that distinct values have distinct serializations. They
imply:

* `Bincode.toBytes_of_fromBytes`: every accepted input is the serialization of the value it
  decodes to (canonicity);
* `Bincode.fromBytes_of_toBytes`: every serialization is accepted, and decodes to a value with
  the same serialization;
* `Bincode.fromBytes_eq_ok_iff`: for injective types, `fromBytes b = .ok x ↔ toBytes x = .ok b`.

Types containing hash maps are neither injective nor canonical: two hash maps with the same
entries may be different Lean values, and Bincode accepts their entries in any order.
`EquivSerialize α` provides an equivalence relation on `α`, which compares hash maps by their
contents, such that every serialization decodes to an equivalent value
(`Bincode.fromBytes_of_toBytes_equiv`), and values with the same serialization are
equivalent (`Bincode.equiv_of_toBytes`). Every injective type has one, using equality.

Floating-point numbers have no instances: their conversions to and from bits are opaque to
Lean.
-/

namespace Bincode

open Serde Std

/-- The `Serialize` and `Deserialize` instances of `α` are correct for every depth budget. -/
class LawfulSerialize (α : Type) [Serialize α] [Deserialize α] : Prop where
  correct : ∀ d, Correct d (serialize (α := α)) (deserialize (α := α))

/-- Distinct values of `α` have distinct serializations. -/
class InjectiveSerialize (α : Type) [Serialize α] : Prop where
  injective : ∀ d, Injective d (serialize (α := α))

/--
Equivalence relations on `α`, indexed by the container-depth budget, such that the
serialization of every value (with that budget) is decoded to an equivalent value. The
relation compares hash maps by their contents, and other values structurally. For types
without hash maps, it is equality.
-/
class EquivSerialize (α : Type) [Serialize α] [Deserialize α] where
  /-- The equivalence relations. -/
  equiv : Nat → α → α → Prop
  equivalence : ∀ d, Equivalence (equiv d)
  roundTrip : ∀ d, RoundTrip (equiv d) d (serialize (α := α)) (deserialize (α := α))

/-- Equivalence of values, as used by `Bincode.toBytes` (i.e. with the full depth budget). -/
abbrev Equivalent {α : Type} [Serialize α] [Deserialize α] [EquivSerialize α] (x y : α) : Prop :=
  EquivSerialize.equiv maxContainerDepth x y

/-! ## Main theorems -/

section

variable {α : Type} [Serialize α] [Deserialize α]

/-- Canonicity: an input accepted by `fromBytes` is the serialization of the decoded value. -/
theorem toBytes_of_fromBytes [LawfulSerialize α] {b : ByteArray} {x : α}
    (h : fromBytes b = .ok x) : toBytes x = .ok b :=
  (LawfulSerialize.correct maxContainerDepth).run_canonical h

/-- Every serialization is accepted, and decodes to a value with the same serialization. -/
theorem fromBytes_of_toBytes [LawfulSerialize α] {x : α} {b : ByteArray}
    (h : toBytes x = .ok b) : ∃ y : α, fromBytes b = .ok y ∧ toBytes y = .ok b := by
  have hc := LawfulSerialize.correct (α := α) maxContainerDepth
  obtain ⟨y, hy, -⟩ := (hc.roundTrip_of_upTo (r := fun _ _ => True) fun _ _ _ _ _ => trivial).run h
  exact ⟨y, hy, toBytes_of_fromBytes hy⟩

/-- For types without hash maps, `fromBytes` and `toBytes` are inverse of each other. -/
theorem fromBytes_eq_ok_iff [LawfulSerialize α] [InjectiveSerialize α] {b : ByteArray} {x : α} :
    fromBytes b = .ok x ↔ toBytes x = .ok b :=
  (LawfulSerialize.correct maxContainerDepth).run_eq_ok_iff
    (InjectiveSerialize.injective maxContainerDepth)

/-- Every value has at most one accepted encoding. -/
theorem fromBytes_injective [LawfulSerialize α] {b₁ b₂ : ByteArray} {x : α}
    (h₁ : fromBytes b₁ = .ok x) (h₂ : fromBytes b₂ = .ok x) : b₁ = b₂ := by
  have := (toBytes_of_fromBytes h₁).symm.trans (toBytes_of_fromBytes h₂)
  injection this

/-- Every serialization is accepted, and decodes to an equivalent value. -/
theorem fromBytes_of_toBytes_equiv [EquivSerialize α] {x : α} {b : ByteArray}
    (h : toBytes x = .ok b) : ∃ y, fromBytes b = .ok y ∧ Equivalent y x :=
  (EquivSerialize.roundTrip maxContainerDepth).run h

/-- Values with the same serialization are equivalent. -/
theorem equiv_of_toBytes [EquivSerialize α] {x y : α} {b : ByteArray}
    (hx : toBytes x = .ok b) (hy : toBytes y = .ok b) : Equivalent x y :=
  ((EquivSerialize.roundTrip maxContainerDepth).injectiveUpTo
    (EquivSerialize.equivalence maxContainerDepth)).run
    (EquivSerialize.roundTrip maxContainerDepth).uniform hx hy

end

/-! ## Instances -/

instance : LawfulSerialize Unit := ⟨fun _ => correct_unit.1⟩
instance : LawfulSerialize Bool := ⟨fun _ => correct_bool.1⟩
instance : LawfulSerialize UInt8 := ⟨fun _ => correct_u8.1⟩
instance : LawfulSerialize UInt16 := ⟨fun _ => correct_u16.1⟩
instance : LawfulSerialize UInt32 := ⟨fun _ => correct_u32.1⟩
instance : LawfulSerialize UInt64 := ⟨fun _ => correct_u64.1⟩
instance : LawfulSerialize UInt128 := ⟨fun _ => correct_u128.1⟩
instance : LawfulSerialize Int8 := ⟨fun _ => correct_i8.1⟩
instance : LawfulSerialize Int16 := ⟨fun _ => correct_i16.1⟩
instance : LawfulSerialize Int32 := ⟨fun _ => correct_i32.1⟩
instance : LawfulSerialize Int64 := ⟨fun _ => correct_i64.1⟩
instance : LawfulSerialize Int128 := ⟨fun _ => correct_i128.1⟩
instance : LawfulSerialize Char := ⟨fun _ => correct_char.1⟩
instance : LawfulSerialize String := ⟨fun _ => correct_str.1⟩
instance : LawfulSerialize ByteArray := ⟨fun _ => correct_bytes.1⟩

instance : InjectiveSerialize Unit := ⟨fun _ => correct_unit.2⟩
instance : InjectiveSerialize Bool := ⟨fun _ => correct_bool.2⟩
instance : InjectiveSerialize UInt8 := ⟨fun _ => correct_u8.2⟩
instance : InjectiveSerialize UInt16 := ⟨fun _ => correct_u16.2⟩
instance : InjectiveSerialize UInt32 := ⟨fun _ => correct_u32.2⟩
instance : InjectiveSerialize UInt64 := ⟨fun _ => correct_u64.2⟩
instance : InjectiveSerialize UInt128 := ⟨fun _ => correct_u128.2⟩
instance : InjectiveSerialize Int8 := ⟨fun _ => correct_i8.2⟩
instance : InjectiveSerialize Int16 := ⟨fun _ => correct_i16.2⟩
instance : InjectiveSerialize Int32 := ⟨fun _ => correct_i32.2⟩
instance : InjectiveSerialize Int64 := ⟨fun _ => correct_i64.2⟩
instance : InjectiveSerialize Int128 := ⟨fun _ => correct_i128.2⟩
instance : InjectiveSerialize Char := ⟨fun _ => correct_char.2⟩
instance : InjectiveSerialize String := ⟨fun _ => correct_str.2⟩
instance : InjectiveSerialize ByteArray := ⟨fun _ => correct_bytes.2⟩

instance (priority := high) {α : Type} [Serialize α] [Deserialize α] [LawfulSerialize α]
    [InjectiveSerialize α] : EquivSerialize α :=
  ⟨fun _ => Eq, fun _ => Rel.eq_equivalence,
    fun d => (LawfulSerialize.correct d).roundTrip (InjectiveSerialize.injective d)⟩

section

variable {K V α β : Type} [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
  [Serialize K] [Deserialize K] [Serialize V] [Deserialize V]
  [Serialize α] [Deserialize α] [Serialize β] [Deserialize β]

instance [LawfulSerialize α] : LawfulSerialize (Option α) :=
  ⟨fun d => correct_option (LawfulSerialize.correct d)⟩

instance [LawfulSerialize α] [InjectiveSerialize α] : InjectiveSerialize (Option α) :=
  ⟨fun d => injective_option (LawfulSerialize.correct d) (InjectiveSerialize.injective d)⟩

instance [LawfulSerialize α] : LawfulSerialize (Array α) :=
  ⟨fun d => correct_seq (LawfulSerialize.correct d)⟩

instance [LawfulSerialize α] [InjectiveSerialize α] : InjectiveSerialize (Array α) :=
  ⟨fun d => injective_seq (LawfulSerialize.correct d) (InjectiveSerialize.injective d)⟩

instance [LawfulSerialize α] : LawfulSerialize (Vector α n) :=
  ⟨fun d => correct_vector (LawfulSerialize.correct d) n⟩

instance [LawfulSerialize α] [InjectiveSerialize α] : InjectiveSerialize (Vector α n) :=
  ⟨fun d => injective_vector (LawfulSerialize.correct d) (InjectiveSerialize.injective d) n⟩

instance [LawfulSerialize α] [LawfulSerialize β] : LawfulSerialize (α × β) :=
  ⟨fun d => (LawfulSerialize.correct d).prod (LawfulSerialize.correct d)⟩

instance [LawfulSerialize α] [LawfulSerialize β] [InjectiveSerialize α] [InjectiveSerialize β] :
    InjectiveSerialize (α × β) :=
  ⟨fun d => Injective.prod (LawfulSerialize.correct d) (LawfulSerialize.correct d)
    (InjectiveSerialize.injective d) (InjectiveSerialize.injective d)⟩

instance [LawfulSerialize K] [InjectiveSerialize K] [EquivSerialize V] :
    EquivSerialize (HashMap K V) :=
  ⟨fun d => Rel.hashMap (EquivSerialize.equiv d),
    fun d => Rel.hashMap_equivalence (EquivSerialize.equivalence d),
    fun d => roundTrip_hashMap
      ((LawfulSerialize.correct d).roundTrip (InjectiveSerialize.injective d))
      (EquivSerialize.roundTrip d)⟩

instance (priority := low) [EquivSerialize α] : EquivSerialize (Option α) :=
  ⟨fun d => Option.Rel (EquivSerialize.equiv d),
    fun d => Rel.option_equivalence (EquivSerialize.equivalence d),
    fun d => roundTrip_option (EquivSerialize.roundTrip d)⟩

instance (priority := low) [EquivSerialize α] : EquivSerialize (Array α) :=
  ⟨fun d => Rel.array (EquivSerialize.equiv d),
    fun d => Rel.array_equivalence (EquivSerialize.equivalence d),
    fun d => roundTrip_seq (EquivSerialize.roundTrip d)⟩

instance (priority := low) [EquivSerialize α] : EquivSerialize (Vector α n) :=
  ⟨fun d => Rel.vector (EquivSerialize.equiv d),
    fun d => Rel.vector_equivalence (EquivSerialize.equivalence d),
    fun d => roundTrip_vector (EquivSerialize.roundTrip d) n⟩

instance (priority := low) [EquivSerialize α] [EquivSerialize β] : EquivSerialize (α × β) :=
  ⟨fun d => Rel.prod (EquivSerialize.equiv d) (EquivSerialize.equiv d),
    fun d => Rel.prod_equivalence (EquivSerialize.equivalence d) (EquivSerialize.equivalence d),
    fun d => (EquivSerialize.roundTrip d).prod (EquivSerialize.roundTrip d)⟩

end

/-- For hash maps of values without hash maps, equivalence is `Std.HashMap.Equiv`. -/
theorem equiv_hashMap_iff {K V : Type} [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    [Serialize K] [Deserialize K] [LawfulSerialize K] [InjectiveSerialize K]
    [Serialize V] [Deserialize V] [LawfulSerialize V] [InjectiveSerialize V]
    {m₁ m₂ : HashMap K V} : Equivalent m₁ m₂ ↔ m₁.Equiv m₂ :=
  Rel.hashMap_eq_iff_equiv

end Bincode
