/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Sequences

/-!
# Correctness of enums and of depth-indexed definitions

Lemmas used by generated code: enums (`Ser.enum` and `De.enum`), their variants, and
definitions that read the depth budget (`Ser.withDepth` and `De.withDepth`), used for
recursive types.

Variant indices are written with a serializer `tag` and read with a deserializer `deTag`,
which must be correct and injective. The hypotheses of the enum lemmas are stated per value
and per variant index, so that the size of the generated proofs is linear in the number of
variants.
-/

namespace Serde

/-! ## Depth-indexed definitions -/

/-- Correctness of definitions that read the depth budget. -/
theorem CorrectOn.withDepth {ser : Nat → α → Ser Unit} {de : Nat → De α}
    (h : CorrectOn P d (ser d) (de d)) :
    CorrectOn P d (fun x => Ser.withDepth fun d => ser d x) (De.withDepth de) :=
  h.congr (fun _ _ => rfl) (fun _ _ => rfl)

theorem InjectiveOn.withDepth {ser : Nat → α → Ser Unit} (h : InjectiveOn P d (ser d)) :
    InjectiveOn P d (fun x => Ser.withDepth fun d => ser d x) :=
  h.congr (fun _ _ => rfl)

/-! ## Enums -/

theorem De.enum_eq_ok {deTag : De UInt32} {payload : Nat → Option (De α)} :
    De.enum deTag name payload ctx pos = .ok y pos' ↔
      ∃ (j : UInt32) (p : Nat) (de : De α), deTag ctx pos = .ok j p ∧
        payload j.toNat = some de ∧ de ctx p = .ok y pos' := by
  simp only [De.enum, De.bind_eq_ok]
  constructor
  · rintro ⟨j, p, hj, h⟩
    cases hpay : payload j.toNat with
    | none =>
      simp only [hpay] at h
      exact absurd h De.throw_ne_ok
    | some de =>
      simp only [hpay] at h
      exact ⟨j, _, de, hj, hpay, h⟩
  · rintro ⟨j, p, de, hj, hpay, h⟩
    refine ⟨j, p, hj, ?_⟩
    simp only [hpay]
    exact h

/--
Correctness of an enum. `hdec` is proved by case analysis on values, and `hcan` by case
analysis on variant indices.
-/
theorem correct_enum {tag : UInt32 → Ser Unit} {deTag : De UInt32}
    (hu : Correct d tag deTag ∧ Injective d tag) {index : α → UInt32} {payload : α → Ser Unit}
    {dePayload : Nat → Option (De α)}
    (hdec : ∀ x, ∃ de, dePayload (index x).toNat = some de ∧ (payload x).Uniform d ∧
      ∀ e, (payload x).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
        input.extract pos (pos + e.size) = e → ∃ y, de ⟨input, d⟩ pos = .ok y (pos + e.size))
    (hcan : ∀ i de, dePayload i = some de → ∀ input pos y pos', de ⟨input, d⟩ pos = .ok y pos' →
      (index y).toNat = i ∧ pos ≤ pos' ∧ (payload y).Writes d (input.extract pos pos')) :
    Correct d (Ser.enum tag index payload) (De.enum deTag name dePayload) where
  uniform x _ := by
    obtain ⟨_, _, hux, _⟩ := hdec x
    exact (hu.1.uniform (index x) trivial).bind hux
  decode x e _ he input pos hsize hinput := by
    obtain ⟨de, hde, hux, hdecx⟩ := hdec x
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := he.of_bind (hu.1.uniform (index x) trivial) hux
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨j, hj⟩ := hu.1.decode (index x) e₁ trivial h₁ input pos (by omega) hi₁
    obtain ⟨-, -, hwj⟩ := hu.1.canonical input pos j _ hj
    rw [hi₁] at hwj
    have := hu.2 j (index x) e₁ trivial trivial hwj h₁
    subst this
    obtain ⟨y, hy⟩ := hdecx e₂ h₂ input _ (by omega) hi₂
    refine ⟨y, De.enum_eq_ok.mpr ⟨index x, _, de, hj, hde, ?_⟩⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]
  canonical input pos y pos' hy := by
    obtain ⟨j, p, de, hj, hpay, hde⟩ := De.enum_eq_ok.mp hy
    obtain ⟨-, hle₁, hw₁⟩ := hu.1.canonical input pos j p hj
    obtain ⟨hidx, hle₂, hw₂⟩ := hcan _ de hpay input p y pos' hde
    have : index y = j := UInt32.toNat_inj.mp hidx
    subst this
    refine ⟨trivial, Nat.le_trans hle₁ hle₂, ?_⟩
    rw [← extract_concat hle₁ hle₂]
    exact hw₁.bind hw₂

/--
Injectivity of an enum, from the fact that the payload of every value is decoded exactly.
`hexact` is proved by case analysis on values.
-/
theorem injective_enum {tag : UInt32 → Ser Unit} {deTag : De UInt32}
    (hu : Correct d tag deTag ∧ Injective d tag) {index : α → UInt32} {payload : α → Ser Unit}
    {dePayload : Nat → Option (De α)}
    (hexact : ∀ x, ∃ de, dePayload (index x).toNat = some de ∧ (payload x).Uniform d ∧
      ∀ e, (payload x).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
        input.extract pos (pos + e.size) = e → de ⟨input, d⟩ pos = .ok x (pos + e.size)) :
    Injective d (Ser.enum tag index payload) := by
  intro x y e _ _ hx hy
  obtain ⟨de, hde, hcx, hdx⟩ := hexact x
  obtain ⟨de', hde', hcy, hdy⟩ := hexact y
  obtain ⟨ex₁, ex₂, hx₁, hx₂, rfl⟩ := hx.of_bind (hu.1.uniform (index x) trivial) hcx
  obtain ⟨ey₁, ey₂, hy₁, hy₂, heq⟩ := hy.of_bind (hu.1.uniform (index y) trivial) hcy
  obtain ⟨hidx, rfl, rfl⟩ := hu.1.injective_of_prefix hu.2 trivial trivial hx₁ hy₁ heq
  rw [← hidx, hde] at hde'
  cases hde'
  have h₁ := hdx ex₂ hx₂ ex₂ 0 (by omega) (by rw [Nat.zero_add, ByteArray.extract_zero_size])
  have h₂ := hdy ex₂ hy₂ ex₂ 0 (by omega) (by rw [Nat.zero_add, ByteArray.extract_zero_size])
  rw [h₁] at h₂
  injection h₂

/-! ## Variants -/

/-- Decoding the payload of a variant. -/
theorem variant_decode {ser : β → Ser Unit} {de : De β} (h : Correct d ser de) (mk : β → α)
    (b : β) : (ser b).Uniform d ∧ ∀ e, (ser b).Writes d e → ∀ input pos,
      pos + e.size ≤ input.size → input.extract pos (pos + e.size) = e →
      ∃ y, (mk <$> de) ⟨input, d⟩ pos = .ok y (pos + e.size) := by
  refine ⟨h.uniform b trivial, fun e he input pos hsize hinput => ?_⟩
  obtain ⟨b', hb'⟩ := h.decode b e trivial he input pos hsize hinput
  exact ⟨mk b', De.map_eq_ok.mpr ⟨b', hb', rfl⟩⟩

/-- Canonicity of the payload of a variant of index `i`, built with `mk`. -/
theorem variant_canonical {ser : β → Ser Unit} {de : De β} (h : Correct d ser de)
    (mk : β → α) {index : α → UInt32} {payload : α → Ser Unit} {i : Nat}
    (hidx : ∀ b, (index (mk b)).toNat = i) (hpay : ∀ b, payload (mk b) = ser b) :
    ∀ input pos y pos', (mk <$> de) ⟨input, d⟩ pos = .ok y pos' →
      (index y).toNat = i ∧ pos ≤ pos' ∧ (payload y).Writes d (input.extract pos pos') := by
  intro input pos y pos' hy
  obtain ⟨b, hb, rfl⟩ := De.map_eq_ok.mp hy
  obtain ⟨-, hle, hw⟩ := h.canonical input pos b pos' hb
  exact ⟨hidx b, hle, by rw [hpay]; exact hw⟩

/-- The payload of a variant is decoded exactly. -/
theorem variant_exact {ser : β → Ser Unit} {de : De β} (h : Correct d ser de)
    (hi : Injective d ser) (mk : β → α) (b : β) : (ser b).Uniform d ∧
    ∀ e, (ser b).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
      input.extract pos (pos + e.size) = e → (mk <$> de) ⟨input, d⟩ pos = .ok (mk b) (pos + e.size) := by
  refine ⟨h.uniform b trivial, ?_⟩
  intro e he input pos hsize hinput
  obtain ⟨b', hb'⟩ := h.decode b e trivial he input pos hsize hinput
  obtain ⟨-, -, hw⟩ := h.canonical input pos b' _ hb'
  rw [hinput] at hw
  rw [hi b' b e trivial trivial hw he] at hb'
  exact De.map_eq_ok.mpr ⟨b, hb', rfl⟩

end Serde
