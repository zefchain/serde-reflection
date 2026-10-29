/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Primitives

/-!
# Correctness of scalar types

Length-prefixed values, byte arrays, strings, booleans, signed integers, unit, and unsupported
types. Lengths are written with a serializer `len` and read with a deserializer `deLen`,
which must be correct and injective.
-/

namespace Serde

/-! ## Length-prefixed values -/

/--
Correctness of values written as a length followed by a body, where the body is decoded
knowing the length.
-/
theorem correct_lengthPrefixed {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) {size : α → Nat} {body : α → Ser Unit}
    {deBody : Nat → De α} (hb : ∀ n, CorrectOn (fun x => size x = n) d body (deBody n)) :
    Correct d (fun x => do len (size x); body x) (do let n ← deLen; deBody n) where
  uniform x _ :=
    (hlen.1.uniform (size x) trivial).bind ((hb (size x)).uniform x rfl)
  decode x e _ he input pos hsize hinput := by
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
      he.of_bind (hlen.1.uniform (size x) trivial) ((hb (size x)).uniform x rfl)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨n, hn⟩ := hlen.1.decode (size x) e₁ trivial h₁ input pos (by omega) hi₁
    obtain ⟨-, -, hwn⟩ := hlen.1.canonical input pos n _ hn
    rw [hi₁] at hwn
    have := hlen.2 n (size x) e₁ trivial trivial hwn h₁
    subst this
    obtain ⟨y, hy⟩ := (hb (size x)).decode x e₂ rfl h₂ input (pos + e₁.size) (by omega) hi₂
    refine ⟨y, De.bind_eq_ok.mpr ⟨_, _, hn, ?_⟩⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]
  canonical input pos y pos' h := by
    obtain ⟨n, p, hn, hy⟩ := De.bind_eq_ok.mp h
    obtain ⟨-, hle₁, hw₁⟩ := hlen.1.canonical input pos n p hn
    obtain ⟨rfl, hle₂, hw₂⟩ := (hb n).canonical input p y pos' hy
    refine ⟨trivial, Nat.le_trans hle₁ hle₂, ?_⟩
    rw [← extract_concat hle₁ hle₂]
    exact hw₁.bind hw₂

theorem injective_lengthPrefixed {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) {size : α → Nat} {body : α → Ser Unit}
    {deBody : Nat → De α} (hb : ∀ n, CorrectOn (fun x => size x = n) d body (deBody n))
    (hi : ∀ n, InjectiveOn (fun x => size x = n) d body) :
    Injective d (fun x => do len (size x); body x) := by
  intro x y e _ _ hx hy
  obtain ⟨ex₁, ex₂, hx₁, hx₂, rfl⟩ :=
    hx.of_bind (hlen.1.uniform (size x) trivial) ((hb (size x)).uniform x rfl)
  obtain ⟨ey₁, ey₂, hy₁, hy₂, heq⟩ :=
    hy.of_bind (hlen.1.uniform (size y) trivial) ((hb (size y)).uniform y rfl)
  obtain ⟨hsize, rfl, rfl⟩ :=
    hlen.1.injective_of_prefix hlen.2 trivial trivial hx₁ hy₁ heq
  exact hi (size x) x y ex₂ rfl hsize.symm hx₂ hy₂

/-! ## Byte arrays and strings -/

theorem correct_raw (n : Nat) : CorrectOn (fun b : ByteArray => b.size = n) d Ser.raw (De.raw n) where
  uniform _ _ := Ser.writes_raw.uniform
  decode b e hb he input pos hsize hinput := by
    have := Ser.writes_raw.unique he
    subst this hb
    refine ⟨input.extract pos (pos + b.size), ?_⟩
    simp only [De.raw, De.bind_eq_ok, De.advance_eq_ok, De.input_eq_ok, De.pure_eq_ok]
    exact ⟨pos, _, ⟨hsize, rfl, rfl⟩, input, _, ⟨rfl, rfl⟩, rfl, rfl⟩
  canonical input pos b pos' h := by
    simp only [De.raw, De.bind_eq_ok, De.advance_eq_ok, De.input_eq_ok, De.pure_eq_ok] at h
    obtain ⟨p, p', ⟨hle, rfl, rfl⟩, i, p'', ⟨rfl, rfl⟩, rfl, rfl⟩ := h
    refine ⟨?_, by omega, Ser.writes_raw⟩
    simp only [ByteArray.size_extract]
    omega

theorem injective_raw (n : Nat) : InjectiveOn (fun b : ByteArray => b.size = n) d Ser.raw :=
  fun _ _ _ _ _ hx hy => Ser.writes_raw.unique hx |>.trans (Ser.writes_raw.unique hy).symm

theorem correct_bytes {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) :
    Correct d (Ser.bytes len) (De.bytes deLen) ∧ Injective d (Ser.bytes len) :=
  ⟨correct_lengthPrefixed hlen (size := ByteArray.size) correct_raw,
    injective_lengthPrefixed hlen (size := ByteArray.size) correct_raw injective_raw⟩

theorem String.fromUTF8?_toUTF8 (s : String) : String.fromUTF8? s.toUTF8 = some s := by
  simp [String.fromUTF8?, String.toUTF8_eq_toByteArray, String.isValidUTF8, String.fromUTF8]

theorem String.toUTF8_of_fromUTF8? {b : ByteArray} (h : String.fromUTF8? b = some s) :
    s.toUTF8 = b := by
  simp only [String.fromUTF8?] at h
  split at h
  · cases h
    simp [String.fromUTF8, String.toUTF8_eq_toByteArray]
  · cases h

theorem correct_str {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) :
    Correct d (Ser.str len) (De.str deLen) ∧ Injective d (Ser.str len) :=
  (correct_bytes hlen).1.filterMap (correct_bytes hlen).2 String.toUTF8 String.fromUTF8? (fun _ => trivial)
    String.fromUTF8?_toUTF8 (fun _ _ h => String.toUTF8_of_fromUTF8? h)
    (fun input pos y pos' => by
      show (match De.bytes deLen ⟨input, d⟩ pos with
        | .ok b pos =>
          match String.fromUTF8? b with
          | some s => EStateM.Result.ok s pos
          | none => .error Error.invalidUtf8 pos
        | .error e pos => .error e pos) = _ ↔ _
      cases hb : De.bytes deLen ⟨input, d⟩ pos with
      | error e p => simp
      | ok b p =>
        show (match String.fromUTF8? b with
          | some s => EStateM.Result.ok s p
          | none => .error Error.invalidUtf8 p) = _ ↔ _
        constructor
        · intro h
          cases hs : String.fromUTF8? b with
          | none => rw [hs] at h; cases h
          | some s =>
            rw [hs] at h
            cases h
            exact ⟨b, rfl, hs⟩
        · rintro ⟨a, ha, h⟩
          cases ha
          rw [h])

/-! ## Booleans, signed integers, unit -/

theorem correct_bool : Correct d Ser.bool De.bool ∧ Injective d Ser.bool :=
  correct_u8.1.filterMap correct_u8.2 (fun x => if x then 1 else 0)
    (fun b => if b = 0 then some false else if b = 1 then some true else none)
    (fun _ => trivial) (fun x => by cases x <;> rfl)
    (fun a y h => by
      split at h
      · cases h; simp_all
      · split at h
        · cases h; simp_all
        · cases h)
    (fun input pos y pos' => by
      show (match De.u8 ⟨input, d⟩ pos with
        | .ok b pos => if b = 0 then EStateM.Result.ok false pos
          else if b = 1 then .ok true pos else .error (Error.invalidBool b) pos
        | .error e pos => .error e pos) = _ ↔ _
      cases hb : De.u8 ⟨input, d⟩ pos with
      | error e p => simp
      | ok b p =>
        simp only [EStateM.Result.ok.injEq]
        by_cases h₀ : b = 0
        · subst h₀
          simp only [↓reduceIte, EStateM.Result.ok.injEq]
          constructor
          · rintro ⟨rfl, rfl⟩
            exact ⟨0, ⟨rfl, rfl⟩, rfl⟩
          · rintro ⟨a, ⟨rfl, rfl⟩, h⟩
            exact ⟨by simpa [eq_comm] using h, rfl⟩
        · by_cases h₁ : b = 1
          · subst h₁
            simp only [↓reduceIte, EStateM.Result.ok.injEq, h₀]
            constructor
            · rintro ⟨rfl, rfl⟩
              exact ⟨1, ⟨rfl, rfl⟩, rfl⟩
            · rintro ⟨a, ⟨rfl, rfl⟩, h⟩
              exact ⟨by simpa [eq_comm] using h, rfl⟩
          · simp [h₀, h₁])

theorem correct_i8 : Correct d Ser.i8 De.i8 ∧ Injective d Ser.i8 :=
  ⟨correct_u8.1.map Int8.toUInt8 UInt8.toInt8 UInt8.toUInt8_toInt8,
    correct_u8.2.map Int8.toUInt8 UInt8.toInt8 Int8.toInt8_toUInt8⟩

theorem correct_i16 : Correct d Ser.i16 De.i16 ∧ Injective d Ser.i16 :=
  ⟨correct_u16.1.map Int16.toUInt16 UInt16.toInt16 UInt16.toUInt16_toInt16,
    correct_u16.2.map Int16.toUInt16 UInt16.toInt16 Int16.toInt16_toUInt16⟩

theorem correct_i32 : Correct d Ser.i32 De.i32 ∧ Injective d Ser.i32 :=
  ⟨correct_u32.1.map Int32.toUInt32 UInt32.toInt32 UInt32.toUInt32_toInt32,
    correct_u32.2.map Int32.toUInt32 UInt32.toInt32 Int32.toInt32_toUInt32⟩

theorem correct_i64 : Correct d Ser.i64 De.i64 ∧ Injective d Ser.i64 :=
  ⟨correct_u64.1.map Int64.toUInt64 UInt64.toInt64 UInt64.toUInt64_toInt64,
    correct_u64.2.map Int64.toUInt64 UInt64.toInt64 Int64.toInt64_toUInt64⟩

theorem correct_i128 : Correct d Ser.i128 De.i128 ∧ Injective d Ser.i128 :=
  ⟨correct_u128.1.map Int128.toUInt128 Int128.ofUInt128 (fun _ => rfl),
    correct_u128.2.map Int128.toUInt128 Int128.ofUInt128 (fun _ => rfl)⟩

theorem correct_unit : Correct d Ser.unit De.unit ∧ Injective d Ser.unit := by
  refine ⟨⟨fun _ _ => (Ser.writes_pure d).uniform, ?_, ?_⟩, fun _ _ _ _ _ _ _ => rfl⟩
  · intro x e _ he input pos _ _
    rw [← (Ser.writes_pure d).unique he]
    exact ⟨(), rfl⟩
  · intro input pos y pos' h
    obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
    refine ⟨trivial, Nat.le_refl _, ?_⟩
    rw [ByteArray.extract_same]
    exact Ser.writes_pure d

/-! ## Unsupported types -/

/-- Unsupported types: serialization and deserialization always fail. -/
theorem correct_unsupported {ser : α → Ser Unit} {de : De α}
    (hser : ∀ x, (ser x).Fails d) (hde : ∀ ctx pos y pos', de ctx pos ≠ .ok y pos') :
    Correct d ser de ∧ Injective d ser :=
  ⟨⟨fun x _ => (hser x).uniform, fun x _ _ he => absurd (hser x) he.not_fails,
    fun _ _ _ _ h => absurd h (hde _ _ _ _)⟩, fun x _ _ _ _ hx => absurd (hser x) hx.not_fails⟩

end Serde
