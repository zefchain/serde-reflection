/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bincode.Basic
import Serde.Lawful

/-!
# Correctness of scalar types

Lengths, which Bincode writes as 64-bit integers (up to `Bincode.maxSequenceLength`), and the
types whose encodings depend on them: sequences, byte arrays, and strings. Also characters,
written in UTF-8.

Floating-point numbers have no proofs: their conversions to and from bits are opaque to Lean.
-/

namespace Bincode

open Serde

/-! ## Lengths and variant indices -/

theorem De.len_eq_ok :
    De.len ctx pos = .ok n pos' ↔
      ∃ u, De.u64 ctx pos = .ok u pos' ∧ u.toNat ≤ maxSequenceLength ∧ n = u.toNat := by
  simp only [De.len, De.bind_eq_ok]
  constructor
  · rintro ⟨u, p, hu, h⟩
    split at h
    · obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
      exact ⟨u, hu, by assumption, rfl⟩
    · exact absurd h De.throw_ne_ok
  · rintro ⟨u, hu, hle, rfl⟩
    exact ⟨u, pos', hu, by rw [ite_eq_left hle]; rfl⟩

theorem correct_len : Correct d Ser.len De.len ∧ Injective d Ser.len := by
  have hu := correct_u64 (d := d)
  have hmax : maxSequenceLength < 2 ^ 64 := by decide
  have hw : ∀ n, n ≤ maxSequenceLength → ∀ e,
      (Ser.len n).Writes d e ↔ (Ser.u64 n.toUInt64).Writes d e :=
    fun n hn e => by rw [Ser.len, ite_eq_left hn]
  have hf : ∀ n, ¬ n ≤ maxSequenceLength → (Ser.len n).Fails d :=
    fun n hn => by rw [Ser.len, ite_eq_right hn]; exact Ser.fails_throw
  refine ⟨⟨fun n _ => ?_, ?_, ?_⟩, ?_⟩
  · by_cases hn : n ≤ maxSequenceLength
    · rw [Ser.len, ite_eq_left hn]
      exact hu.1.uniform _ trivial
    · exact (hf n hn).uniform
  · intro n e _ he input pos hsize hinput
    by_cases hn : n ≤ maxSequenceLength
    · obtain ⟨u, hu', rfl⟩ := (hu.1.roundTrip hu.2).decode _ e ((hw n hn e).mp he) input pos
        hsize hinput
      have : n.toUInt64.toNat = n := by simp; omega
      exact ⟨n, De.len_eq_ok.mpr ⟨_, hu', by omega, this.symm⟩⟩
    · exact absurd (hf n hn) he.not_fails
  · intro input pos n pos' h
    obtain ⟨u, hu', hle, rfl⟩ := De.len_eq_ok.mp h
    obtain ⟨-, hle', hwu⟩ := hu.1.canonical input pos u pos' hu'
    refine ⟨trivial, hle', (hw u.toNat hle _).mpr ?_⟩
    simpa using hwu
  · intro m n e _ _ hm hn
    by_cases hm' : m ≤ maxSequenceLength
    · by_cases hn' : n ≤ maxSequenceLength
      · have := hu.2 _ _ e trivial trivial ((hw m hm' e).mp hm) ((hw n hn' e).mp hn)
        have := congrArg UInt64.toNat this
        simp only [Nat.toUInt64_eq, UInt64.toNat_ofNat'] at this
        omega
      · exact absurd (hf n hn') hn.not_fails
    · exact absurd (hf m hm') hm.not_fails

/-- Variant indices of enums, used with `Serde.correct_enum`. -/
theorem correct_variantIndex : Correct d Ser.u32 De.u32 ∧ Injective d Ser.u32 :=
  correct_u32

/-! ## Sequences, byte arrays, and strings -/

theorem correct_bytes : Correct d Ser.bytes De.bytes ∧ Injective d Ser.bytes :=
  Serde.correct_bytes correct_len

theorem correct_str : Correct d Ser.str De.str ∧ Injective d Ser.str :=
  Serde.correct_str correct_len

theorem correct_seq {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) :
    Correct d (Ser.seq ser) (De.seq de) :=
  Serde.correct_seq correct_len h

theorem injective_seq {ser : α → Ser Unit} {de : De α} (h : Correct d ser de)
    (hi : Injective d ser) : Injective d (Ser.seq ser) :=
  Serde.injective_seq correct_len h hi

theorem roundTrip_seq {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de) :
    RoundTrip (Rel.array r) d (Ser.seq ser) (De.seq de) :=
  Serde.roundTrip_seq correct_len h

/-! ## Characters -/

theorem De.char_eq_ok :
    De.char ctx pos = .ok c pos' ↔ ctx.input.utf8DecodeChar? pos = some c ∧
      pos' = pos + c.utf8Size := by
  show (match ctx.input.utf8DecodeChar? pos with
    | some c => EStateM.Result.ok c (pos + c.utf8Size)
    | none => .error Error.invalidChar pos) = _ ↔ _
  split <;> simp_all [eq_comm]

theorem size_utf8EncodeChar (c : Char) :
    (String.utf8EncodeChar c).toByteArray.size = c.utf8Size := by
  simp

theorem correct_char : Correct d Ser.char De.char ∧ Injective d Ser.char := by
  refine ⟨⟨fun _ _ => Ser.writes_raw.uniform, ?_, ?_⟩, ?_⟩
  · intro c e _ he input pos hsize hinput
    have := Ser.writes_raw.unique he
    subst this
    refine ⟨c, De.char_eq_ok.mpr ⟨?_, by rw [size_utf8EncodeChar]⟩⟩
    rw [ByteArray.utf8DecodeChar?_eq_utf8DecodeChar?_extract,
      ← extract_concat (Nat.le_add_right pos _) hsize, hinput]
    exact ByteArray.utf8DecodeChar?_utf8EncodeChar_append
  · intro input pos c pos' h
    obtain ⟨hc, rfl⟩ := De.char_eq_ok.mp h
    dsimp only at hc
    have hle := ByteArray.le_size_of_utf8DecodeChar?_eq_some hc
    rw [ByteArray.utf8DecodeChar?_eq_utf8DecodeChar?_extract] at hc
    have heq : input.extract pos (pos + c.utf8Size) = (String.utf8EncodeChar c).toByteArray := by
      rw [String.toByteArray_utf8EncodeChar_of_utf8DecodeChar?_eq_some hc,
        ByteArray.extract_extract, Nat.add_zero, Nat.min_eq_left hle]
    refine ⟨trivial, Nat.le_add_right _ _, ?_⟩
    rw [heq]
    exact Ser.writes_raw
  · intro c c' e _ _ hc hc'
    have h := (Ser.writes_raw.unique hc).trans (Ser.writes_raw.unique hc').symm
    have h₁ := ByteArray.utf8DecodeChar?_utf8EncodeChar_append (c := c) (b := .empty)
    rw [h, ByteArray.utf8DecodeChar?_utf8EncodeChar_append] at h₁
    exact (Option.some_inj.mp h₁).symm

end Bincode
