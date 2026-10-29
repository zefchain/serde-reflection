/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs.Lawful.Uleb128

/-!
# Correctness of scalar types

Lengths and variant indices, which BCS writes in ULEB128 format, and the types whose
encodings depend on them: sequences, byte arrays, and strings. Also the types that BCS does
not support.
-/

namespace Bcs

open Serde

/-! ## Lengths and 32-bit integers -/

theorem De.len_eq_ok :
    De.len ctx pos = .ok n pos' ↔ De.ulebNat 5 ctx pos = .ok n pos' ∧ n ≤ maxSequenceLength := by
  simp only [De.len, De.bind_eq_ok]
  constructor
  · rintro ⟨a, p, ha, h⟩
    split at h
    · obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
      exact ⟨ha, by assumption⟩
    · exact absurd h De.throw_ne_ok
  · rintro ⟨h, hle⟩
    exact ⟨n, pos', h, by rw [ite_eq_left hle]; rfl⟩

theorem correct_len : Correct d Ser.len De.len ∧ Injective d Ser.len := by
  have hmax : maxSequenceLength < 2 ^ 35 := by decide
  have hw : ∀ n, n ≤ maxSequenceLength → (Ser.len n).Writes d (ulebBytes n).toByteArray :=
    fun n hn => by rw [Ser.len, ite_eq_left hn]; exact Ser.writes_ulebNat n
  have hf : ∀ n, ¬ n ≤ maxSequenceLength → (Ser.len n).Fails d :=
    fun n hn => by rw [Ser.len, ite_eq_right hn]; exact Ser.fails_throw
  refine ⟨⟨fun n _ => ?_, ?_, ?_⟩, ?_⟩
  · by_cases hn : n ≤ maxSequenceLength
    · exact (hw n hn).uniform
    · exact (hf n hn).uniform
  · intro n e _ he input pos hsize hinput
    by_cases hn : n ≤ maxSequenceLength
    · have he' := (hw n hn).unique he
      subst he'
      obtain ⟨v, hv⟩ := (correct_ulebNat (d := d)).1.decode n _ (by omega) (Ser.writes_ulebNat n) input pos
        hsize hinput
      obtain ⟨hv', -, hwv⟩ := (correct_ulebNat (d := d)).1.canonical input pos v _ hv
      rw [hinput] at hwv
      have := (correct_ulebNat (d := d)).2 v n _ hv' (by omega) hwv (Ser.writes_ulebNat n)
      subst this
      exact ⟨v, De.len_eq_ok.mpr ⟨hv, hn⟩⟩
    · exact absurd (hf n hn) he.not_fails
  · intro input pos n pos' h
    obtain ⟨h, hn⟩ := De.len_eq_ok.mp h
    obtain ⟨-, hle, hwn⟩ := (correct_ulebNat (d := d)).1.canonical input pos n pos' h
    refine ⟨trivial, hle, ?_⟩
    rw [Ser.len, ite_eq_left hn]
    exact hwn
  · intro m n e _ _ hm hn
    by_cases hm' : m ≤ maxSequenceLength
    · by_cases hn' : n ≤ maxSequenceLength
      · exact (correct_ulebNat (d := d)).2 m n e (by omega) (by omega)
          (by rw [Ser.len, ite_eq_left hm'] at hm; exact hm)
          (by rw [Ser.len, ite_eq_left hn'] at hn; exact hn)
      · exact absurd (hf n hn') hn.not_fails
    · exact absurd (hf m hm') hm.not_fails

theorem correct_uleb128 : Correct d Ser.uleb128 De.uleb128 ∧ Injective d Ser.uleb128 :=
  correct_ulebNat.1.filterMap correct_ulebNat.2 UInt32.toNat
    (fun n => if n < 2 ^ 32 then some n.toUInt32 else none)
    (fun x => Nat.lt_trans x.toNat_lt (by decide))
    (fun x => by simp [x.toNat_lt])
    (fun a y h => by
      split at h
      · cases h
        simp only [Nat.toUInt32]
        exact Nat.mod_eq_of_lt (by assumption)
      · cases h)
    (fun input pos y pos' => by
      simp only [De.uleb128, De.bind_eq_ok]
      constructor
      · rintro ⟨a, p, ha, h⟩
        refine ⟨a, ?_, ?_⟩
        · split at h
          · obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
            exact ha
          · exact absurd h De.throw_ne_ok
        · split at h
          · obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
            simp [*]
          · exact absurd h De.throw_ne_ok
      · rintro ⟨a, ha, h⟩
        refine ⟨a, pos', ha, ?_⟩
        split at h
        · cases h
          rw [ite_eq_left (by assumption)]
          rfl
        · cases h)

/-- Variant indices of enums, used with `Serde.correct_enum`. -/
theorem correct_variantIndex : Correct d Ser.uleb128 De.uleb128 ∧ Injective d Ser.uleb128 :=
  correct_uleb128

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

/-! ## Unsupported types -/

theorem correct_char : Correct d Ser.char De.char ∧ Injective d Ser.char :=
  correct_unsupported (fun _ => Ser.fails_throw) (fun _ _ _ _ => De.throw_ne_ok)

theorem correct_f32 : Correct d Ser.f32 De.f32 ∧ Injective d Ser.f32 :=
  correct_unsupported (fun _ => Ser.fails_throw) (fun _ _ _ _ => De.throw_ne_ok)

theorem correct_f64 : Correct d Ser.f64 De.f64 ∧ Injective d Ser.f64 :=
  correct_unsupported (fun _ => Ser.fails_throw) (fun _ _ _ _ => De.throw_ne_ok)

end Bcs
