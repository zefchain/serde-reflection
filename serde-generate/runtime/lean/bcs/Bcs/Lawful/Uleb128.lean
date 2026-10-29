/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs.Basic
import Serde.Lawful

/-!
# Correctness of ULEB128 integers

ULEB128 is used for the lengths of sequences and for the indices of enum variants. The
decoder rejects redundant trailing zeros, so that every integer has a unique encoding.
-/

namespace Bcs

open Serde

/-- The ULEB128 encoding of a natural number. -/
def ulebBytes (n : Nat) : List UInt8 :=
  if n < 0x80 then [n.toUInt8] else (n % 0x80 + 0x80).toUInt8 :: ulebBytes (n / 0x80)
termination_by n
decreasing_by omega

theorem ulebBytes_of_lt (h : n < 0x80) : ulebBytes n = [n.toUInt8] := by
  rw [ulebBytes, ite_eq_left h]

theorem ulebBytes_of_ge (h : ¬ n < 0x80) :
    ulebBytes n = (n % 0x80 + 0x80).toUInt8 :: ulebBytes (n / 0x80) := by
  rw [ulebBytes, ite_eq_right h]

theorem length_ulebBytes_pos (n : Nat) : 0 < (ulebBytes n).length := by
  rw [ulebBytes]
  split <;> simp

theorem length_ulebBytes_le (h : n < 0x80 ^ (k + 1)) : (ulebBytes n).length ≤ k + 1 := by
  induction k generalizing n with
  | zero => simp [ulebBytes_of_lt (by simpa using h)]
  | succ k ih =>
    by_cases hn : n < 0x80
    · simp [ulebBytes_of_lt hn]
    · rw [ulebBytes_of_ge hn, List.length_cons]
      have : n / 0x80 < 0x80 ^ (k + 1) := by
        rw [Nat.div_lt_iff_lt_mul (by decide), ← Nat.pow_succ]
        exact h
      have := ih this
      omega

theorem lt_of_length_ulebBytes_le (h : (ulebBytes n).length ≤ k + 1) : n < 0x80 ^ (k + 1) := by
  induction k generalizing n with
  | zero =>
    by_cases hn : n < 0x80
    · simpa using hn
    · rw [ulebBytes_of_ge hn, List.length_cons] at h
      have := length_ulebBytes_pos (n / 0x80)
      omega
  | succ k ih =>
    by_cases hn : n < 0x80
    · exact Nat.lt_of_lt_of_le hn (Nat.le_self_pow (by omega) _)
    · rw [ulebBytes_of_ge hn, List.length_cons] at h
      have := ih (n := n / 0x80) (by omega)
      rw [Nat.pow_succ]
      omega

theorem Ser.writes_ulebNat (n : Nat) : (Ser.ulebNat n).Writes d (ulebBytes n).toByteArray := by
  induction n using Nat.strongRecOn with
  | _ n ih =>
    rw [Ser.ulebNat]
    by_cases hn : n < 0x80
    · rw [ite_eq_left hn, ulebBytes_of_lt hn]
      exact Ser.writes_u8
    · rw [ite_eq_right hn, ulebBytes_of_ge hn, show ∀ b (l : List UInt8), b :: l = [b] ++ l from fun _ _ => rfl,
        List.toByteArray_append]
      exact Ser.writes_u8.bind (ih _ (by omega))

namespace De

open Serde.De (bind_eq_ok pure_eq_ok u8_eq_ok throw_ne_ok)

theorem ulebNat_succ_eq_ok :
    ulebNat (fuel + 1) ctx pos = .ok v pos' ↔
      pos + 1 ≤ ctx.input.size ∧
        ((ctx.input[pos]! < 0x80 ∧ v = ctx.input[pos]!.toNat ∧ pos' = pos + 1) ∨
          (¬ ctx.input[pos]! < 0x80 ∧ ∃ rest, ulebNat fuel ctx (pos + 1) = .ok rest pos' ∧
            rest ≠ 0 ∧ v = ctx.input[pos]!.toNat - 0x80 + 0x80 * rest)) := by
  simp only [ulebNat, bind_eq_ok, u8_eq_ok]
  constructor
  · rintro ⟨b, p, ⟨hle, rfl, rfl⟩, h⟩
    refine ⟨hle, ?_⟩
    split at h
    · simp only [pure_eq_ok] at h
      exact .inl ⟨by assumption, h.1.symm, h.2.symm⟩
    · obtain ⟨rest, p', hrest, h⟩ := bind_eq_ok.mp h
      split at h
      · exact absurd h throw_ne_ok
      · simp only [pure_eq_ok] at h
        exact .inr ⟨by assumption, rest, h.2 ▸ hrest, by assumption, h.1.symm⟩
  · rintro ⟨hle, ⟨hlt, rfl, rfl⟩ | ⟨hge, rest, hrest, hne, rfl⟩⟩
    · exact ⟨_, _, ⟨hle, rfl, rfl⟩, by rw [ite_eq_left hlt]; rfl⟩
    · refine ⟨_, _, ⟨hle, rfl, rfl⟩, ?_⟩
      rw [ite_eq_right hge]
      exact bind_eq_ok.mpr ⟨rest, pos', hrest, by rw [ite_eq_right hne]; rfl⟩

theorem ulebNat_zero_ne_ok : ulebNat 0 ctx pos ≠ .ok v pos' := throw_ne_ok

end De

theorem UInt8.toNat_toUInt8_of_lt {n : Nat} (h : n < 256) : n.toUInt8.toNat = n := by
  simp [Nat.toUInt8, Nat.mod_eq_of_lt h]

theorem De.ulebNat_decode {n fuel pos : Nat} {input : ByteArray}
    (hfuel : (ulebBytes n).length ≤ fuel) (hsize : pos + (ulebBytes n).length ≤ input.size)
    (h : input.extract pos (pos + (ulebBytes n).length) = (ulebBytes n).toByteArray) :
    De.ulebNat fuel ⟨input, d⟩ pos = .ok n (pos + (ulebBytes n).length) := by
  induction n using Nat.strongRecOn generalizing fuel pos with
  | _ n ih =>
    cases fuel with
    | zero => have := length_ulebBytes_pos n; omega
    | succ fuel =>
      have hb := getElem!_of_extract h 0 (length_ulebBytes_pos n)
      rw [Nat.add_zero] at hb
      apply De.ulebNat_succ_eq_ok.mpr
      simp only
      refine ⟨by have := length_ulebBytes_pos n; omega, ?_⟩
      by_cases hn : n < 0x80
      · simp only [ulebBytes_of_lt hn, List.getElem_cons_zero] at hb ⊢
        rw [hb]
        have : n.toUInt8.toNat = n := UInt8.toNat_toUInt8_of_lt (by omega)
        refine .inl ⟨?_, this.symm, rfl⟩
        rw [UInt8.lt_iff_toNat_lt, this]
        exact hn
      · simp only [ulebBytes_of_ge hn, List.getElem_cons_zero] at hb
        have hbyte : (n % 0x80 + 0x80).toUInt8.toNat = n % 0x80 + 0x80 :=
          UInt8.toNat_toUInt8_of_lt (by omega)
        rw [hb]
        refine .inr ⟨?_, n / 0x80, ?_, by omega, ?_⟩
        · rw [UInt8.lt_iff_toNat_lt, hbyte]
          simp
        · rw [ulebBytes_of_ge hn] at hfuel hsize h
          simp only [List.length_cons] at hfuel hsize h
          rw [show ∀ b (l : List UInt8), b :: l = [b] ++ l from fun _ _ => rfl,
            show (ulebBytes (n / 0x80)).length + 1 = [(n % 0x80 + 0x80).toUInt8].length +
              (ulebBytes (n / 0x80)).length by simp; omega] at h
          have h' := (extract_split' h).2
          simp only [List.length_singleton] at h'
          rw [ulebBytes_of_ge hn, List.length_cons,
            show pos + ((ulebBytes (n / 0x80)).length + 1) =
              pos + 1 + (ulebBytes (n / 0x80)).length by omega]
          exact ih _ (by omega) (by omega) (by omega) h'
        · rw [hbyte]
          omega

theorem De.ulebNat_canonical {fuel pos : Nat} {ctx : DeContext}
    (h : De.ulebNat fuel ctx pos = .ok v pos') :
    pos' = pos + (ulebBytes v).length ∧ (ulebBytes v).length ≤ fuel ∧
      ctx.input.extract pos pos' = (ulebBytes v).toByteArray := by
  induction fuel generalizing pos v pos' with
  | zero => exact absurd h De.ulebNat_zero_ne_ok
  | succ fuel ih =>
    obtain ⟨hle, ⟨hlt, rfl, rfl⟩ | ⟨hge, rest, hrest, hne, rfl⟩⟩ := De.ulebNat_succ_eq_ok.mp h
    · have hv : ctx.input[pos]!.toNat < 0x80 := by
        rw [UInt8.lt_iff_toNat_lt] at hlt
        simpa using hlt
      rw [ulebBytes_of_lt hv]
      refine ⟨rfl, by simp, ?_⟩
      rw [extract_eq_ofFn hle]
      simp
    · obtain ⟨rfl, hfuel, hext⟩ := ih hrest
      have hb : 0x80 ≤ ctx.input[pos]!.toNat := by
        rw [UInt8.lt_iff_toNat_lt] at hge
        simpa using hge
      have hb' := ctx.input[pos]!.toNat_lt
      have hv : ¬ ctx.input[pos]!.toNat - 0x80 + 0x80 * rest < 0x80 := by omega
      have hdiv : (ctx.input[pos]!.toNat - 0x80 + 0x80 * rest) / 0x80 = rest := by omega
      have hmod : ((ctx.input[pos]!.toNat - 0x80 + 0x80 * rest) % 0x80 + 0x80).toUInt8 =
          ctx.input[pos]! := by
        apply UInt8.toNat_inj.mp
        rw [UInt8.toNat_toUInt8_of_lt (by omega)]
        omega
      rw [ulebBytes_of_ge hv, hdiv, hmod]
      refine ⟨by simp; omega, by simp; omega, ?_⟩
      rw [← extract_concat (p₁ := pos + 1) (by omega) (by omega), hext, extract_eq_ofFn hle]
      simp only [List.ofFn_succ, List.ofFn_zero, Fin.val_zero, Nat.add_zero,
        ← List.toByteArray_append, List.singleton_append]

theorem ulebBytes_inj (h : (ulebBytes m).toByteArray = (ulebBytes n).toByteArray) : m = n := by
  have hlen : (ulebBytes m).length = (ulebBytes n).length := by
    simpa [List.size_toByteArray] using congrArg ByteArray.size h
  have hext : ∀ k (l : List UInt8), k = l.length → l.toByteArray.extract 0 (0 + k) = l.toByteArray := by
    intro k l hk
    rw [Nat.zero_add, hk, ← List.size_toByteArray, ByteArray.extract_zero_size]
  have hm := De.ulebNat_decode (d := 0) (fuel := (ulebBytes n).length) (pos := 0)
    (input := (ulebBytes n).toByteArray) (Nat.le_of_eq hlen)
    (by simp [List.size_toByteArray, hlen]) (by rw [hext _ _ hlen, h])
  have hn := De.ulebNat_decode (d := 0) (fuel := (ulebBytes n).length) (pos := 0)
    (input := (ulebBytes n).toByteArray) (Nat.le_refl _) (by simp [List.size_toByteArray])
    (hext _ _ rfl)
  rw [hm] at hn
  injection hn

/-- ULEB128 integers that fit in 5 bytes. -/
theorem correct_ulebNat :
    CorrectOn (· < 2 ^ 35) d Ser.ulebNat (De.ulebNat 5) ∧
      InjectiveOn (· < 2 ^ 35) d Ser.ulebNat := by
  refine ⟨⟨fun n _ => (Ser.writes_ulebNat n).uniform, ?_, ?_⟩, ?_⟩
  · intro n e hn he input pos hsize hinput
    rw [← (Ser.writes_ulebNat n).unique he] at hsize hinput ⊢
    simp only [List.size_toByteArray] at hsize hinput ⊢
    exact ⟨n, De.ulebNat_decode (length_ulebBytes_le (k := 4) (by simpa using hn)) hsize hinput⟩
  · intro input pos v pos' h
    obtain ⟨rfl, hfuel, hext⟩ := De.ulebNat_canonical h
    refine ⟨lt_of_length_ulebBytes_le (k := 4) hfuel, by omega, ?_⟩
    · rw [hext]
      exact Ser.writes_ulebNat v
  · intro m n e _ _ hm hn
    exact ulebBytes_inj (((Ser.writes_ulebNat m).unique hm).trans ((Ser.writes_ulebNat n).unique hn).symm)

end Bcs
