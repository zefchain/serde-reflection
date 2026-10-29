/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Scalars

/-!
# Correctness of options, sequences, and fixed-size arrays
-/

namespace Serde

/-! ## Options -/

theorem De.option_eq_ok {de : De α} :
    De.option de ctx pos = .ok y pos' ↔ ∃ b p, De.u8 ctx pos = .ok b p ∧
      ((b = 0 ∧ y = none ∧ pos' = p) ∨
        (b = 1 ∧ ∃ a, de ctx p = .ok a pos' ∧ y = some a)) := by
  show (match De.u8 ctx pos with
    | .ok b pos => if b = 0 then EStateM.Result.ok none pos
      else if b = 1 then (some <$> de) ctx pos else .error (Error.invalidOptionTag b) pos
    | .error e pos => .error e pos) = _ ↔ _
  constructor
  · intro h
    split at h
    · rename_i b p hb
      refine ⟨b, p, hb, ?_⟩
      by_cases h₀ : b = 0
      · rw [ite_eq_left h₀] at h
        cases h
        exact .inl ⟨h₀, rfl, rfl⟩
      · rw [ite_eq_right h₀] at h
        by_cases h₁ : b = 1
        · rw [ite_eq_left h₁] at h
          obtain ⟨a, ha, rfl⟩ := De.map_eq_ok.mp h
          exact .inr ⟨h₁, a, ha, rfl⟩
        · rw [ite_eq_right h₁] at h
          cases h
    · cases h
  · rintro ⟨b, p, hb, ⟨rfl, rfl, rfl⟩ | ⟨rfl, a, ha, rfl⟩⟩
    · rw [hb]
      rfl
    · rw [hb]
      show (if (1 : UInt8) = 0 then _ else if (1 : UInt8) = 1 then (some <$> de) ctx p else _) = _
      rw [ite_eq_right (by decide), ite_eq_left rfl]
      exact De.map_eq_ok.mpr ⟨a, ha, rfl⟩

theorem correct_option {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) : Correct d (Ser.option ser) (De.option de) where
  uniform x _ := by
    cases x with
    | none => exact (correct_u8 (d := d)).1.uniform 0 trivial
    | some x => exact ((correct_u8 (d := d)).1.uniform 1 trivial).bind (h.uniform x trivial)
  decode x e _ he input pos hsize hinput := by
    cases x with
    | none =>
      have := (Ser.writes_u8 (x := 0)).unique he
      subst this
      obtain ⟨b, hb⟩ := (correct_u8 (d := d)).1.decode 0 _ trivial Ser.writes_u8 input pos hsize hinput
      obtain ⟨-, -, hwb⟩ := (correct_u8 (d := d)).1.canonical input pos b _ hb
      rw [hinput] at hwb
      have := (correct_u8 (d := d)).2 b 0 _ trivial trivial hwb Ser.writes_u8
      subst this
      exact ⟨none, De.option_eq_ok.mpr ⟨0, _, hb, .inl ⟨rfl, rfl, rfl⟩⟩⟩
    | some x =>
      obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
        he.of_bind ((correct_u8 (d := d)).1.uniform 1 trivial) (h.uniform x trivial)
      have := (Ser.writes_u8 (x := 1)).unique h₁
      subst this
      obtain ⟨hi₁, hi₂⟩ := extract_split hinput
      rw [ByteArray.size_append] at hsize
      obtain ⟨b, hb⟩ := (correct_u8 (d := d)).1.decode 1 _ trivial Ser.writes_u8 input pos (by omega) hi₁
      obtain ⟨-, -, hwb⟩ := (correct_u8 (d := d)).1.canonical input pos b _ hb
      rw [hi₁] at hwb
      have := (correct_u8 (d := d)).2 b 1 _ trivial trivial hwb Ser.writes_u8
      subst this
      obtain ⟨a, ha⟩ := h.decode x e₂ trivial h₂ input _ (by omega) hi₂
      refine ⟨some a, De.option_eq_ok.mpr ⟨1, _, hb, .inr ⟨rfl, a, ?_, rfl⟩⟩⟩
      rwa [ByteArray.size_append, ← Nat.add_assoc]
  canonical input pos y pos' hy := by
    obtain ⟨b, p, hb, ⟨rfl, rfl, rfl⟩ | ⟨rfl, a, ha, rfl⟩⟩ := De.option_eq_ok.mp hy
    · obtain ⟨-, hle, hw⟩ := (correct_u8 (d := d)).1.canonical input pos 0 _ hb
      exact ⟨trivial, hle, hw⟩
    · obtain ⟨-, hle₁, hw₁⟩ := (correct_u8 (d := d)).1.canonical input pos 1 p hb
      obtain ⟨-, hle₂, hw₂⟩ := h.canonical input p a pos' ha
      refine ⟨trivial, Nat.le_trans hle₁ hle₂, ?_⟩
      rw [← extract_concat hle₁ hle₂]
      exact hw₁.bind hw₂

theorem injective_option {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (hi : Injective d ser) :
    Injective d (Ser.option ser) := by
  have hu := correct_u8 (d := d)
  intro x y e _ _ hx hy
  cases x with
  | none =>
    cases y with
    | none => rfl
    | some y =>
      obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
        hy.of_bind (hu.1.uniform 1 trivial) (h.uniform y trivial)
      have := hu.1.injective_of_prefix hu.2 trivial trivial h₁ hx
        (r := e₂) (r' := .empty) (by rw [ByteArray.append_empty])
      cases this.1
  | some x =>
    obtain ⟨ex₁, ex₂, hx₁, hx₂, rfl⟩ := hx.of_bind (hu.1.uniform 1 trivial) (h.uniform x trivial)
    cases y with
    | none =>
      have := hu.1.injective_of_prefix hu.2 trivial trivial hx₁ hy
        (r := ex₂) (r' := .empty) (by rw [ByteArray.append_empty])
      cases this.1
    | some y =>
      obtain ⟨ey₁, ey₂, hy₁, hy₂, heq⟩ := hy.of_bind (hu.1.uniform 1 trivial) (h.uniform y trivial)
      obtain ⟨-, rfl, rfl⟩ := hu.1.injective_of_prefix hu.2 trivial trivial hx₁ hy₁ heq
      rw [hi x y ex₂ trivial trivial hx₂ hy₂]

/-! ## Sequences of elements -/

theorem List.forM_cons_ser {f : α → Ser Unit} : (x :: l).forM f = (do f x; l.forM f) := rfl

theorem Ser.writes_forM_nil {f : α → Ser Unit} : (([] : List α).forM f).Writes d .empty :=
  Ser.writes_pure d

theorem Ser.uniform_forM {f : α → Ser Unit} :
    ∀ {l : List α}, (∀ x ∈ l, (f x).Uniform d) → (l.forM f).Uniform d
  | [], _ => Ser.writes_forM_nil.uniform
  | x :: l, h => by
    rw [List.forM_cons_ser]
    exact (h x (List.mem_cons_self ..)).bind
      (Ser.uniform_forM fun y hy => h y (List.mem_cons_of_mem _ hy))

theorem De.elems_zero {de : De α} :
    De.elems de 0 acc ctx pos = .ok r pos' ↔ acc = r ∧ pos = pos' := by
  show EStateM.Result.ok acc pos = _ ↔ _
  simp

theorem De.elems_succ {de : De α} :
    De.elems de (n + 1) acc ctx pos = .ok r pos' ↔
      ∃ a p, de ctx pos = .ok a p ∧ De.elems de n (acc.push a) ctx p = .ok r pos' := by
  show (match de ctx pos with
    | .ok a pos => De.elemsLoop de.toFun ctx n (acc.push a) pos
    | .error e pos => .error e pos) = _ ↔ _
  constructor
  · intro h
    split at h
    · exact ⟨_, _, by assumption, h⟩
    · cases h
  · rintro ⟨a, p, h₁, h₂⟩
    simp only [h₁]
    exact h₂

theorem Ser.elemsFrom_eq {f : α → Ser Unit} {xs : Array α} {d : Nat} :
    ∀ (k i : Nat) (out : ByteArray), xs.size - i = k →
      Ser.elemsFrom f xs d i out = ((xs.toList.drop i).forM f : Ser Unit) d out
  | 0, i, out, hk => by
    rw [Ser.elemsFrom, dite_eq_right (by omega), List.drop_eq_nil_of_le (by simp; omega)]
    rfl
  | k + 1, i, out, hk => by
    have hi : i < xs.size := by omega
    rw [Ser.elemsFrom, dite_eq_left hi, List.drop_eq_getElem_cons (by simpa using hi),
      List.forM_cons_ser, Ser.bind_apply, Array.getElem_toList]
    split
    · next h => rw [h]; exact Ser.elemsFrom_eq k (i + 1) _ (by omega)
    · next h => rw [h]

theorem Ser.elems_eq {f : α → Ser Unit} (xs : Array α) (d : Nat) (out : ByteArray) :
    Ser.elems f xs d out = (xs.toList.forM f : Ser Unit) d out :=
  Ser.elemsFrom_eq _ 0 out rfl

theorem decode_elems {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) :
    ∀ (l : List α) (e : ByteArray) (acc : Array α) (pos : Nat), Ser.Writes (l.forM ser) d e →
      pos + e.size ≤ input.size → input.extract pos (pos + e.size) = e →
      ∃ r, De.elems de l.length acc ⟨input, d⟩ pos = .ok r (pos + e.size)
  | [], e, acc, pos, he, _, _ => by
    rw [← Ser.writes_forM_nil.unique he]
    exact ⟨acc, rfl⟩
  | x :: l, e, acc, pos, he, hsize, hinput => by
    rw [List.forM_cons_ser] at he
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := he.of_bind (h.uniform x trivial)
      (Ser.uniform_forM fun y _ => h.uniform y trivial)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨a, ha⟩ := h.decode x e₁ trivial h₁ input pos (by omega) hi₁
    obtain ⟨r, hr⟩ := decode_elems h l e₂ (acc.push a) (pos + e₁.size) h₂ (by omega) hi₂
    refine ⟨r, De.elems_succ.mpr ⟨a, _, ha, ?_⟩⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]

theorem canonical_elems {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) :
    ∀ (n : Nat) (acc : Array α) (pos : Nat) (r : Array α) (pos' : Nat),
      De.elems de n acc ⟨input, d⟩ pos = .ok r pos' →
      ∃ l : List α, r = acc ++ l.toArray ∧ l.length = n ∧ pos ≤ pos' ∧
        Ser.Writes (l.forM ser) d (input.extract pos pos')
  | 0, acc, pos, r, pos', hr => by
    obtain ⟨rfl, rfl⟩ := De.elems_zero.mp hr
    refine ⟨[], by simp, rfl, Nat.le_refl _, ?_⟩
    rw [ByteArray.extract_same]
    exact Ser.writes_forM_nil
  | n + 1, acc, pos, r, pos', hr => by
    obtain ⟨a, p, ha, hr⟩ := De.elems_succ.mp hr
    obtain ⟨-, hle₁, hw₁⟩ := h.canonical input pos a p ha
    obtain ⟨l, rfl, rfl, hle₂, hw₂⟩ := canonical_elems h n (acc.push a) p r pos' hr
    refine ⟨a :: l, by simp, rfl, Nat.le_trans hle₁ hle₂, ?_⟩
    rw [List.forM_cons_ser, ← extract_concat hle₁ hle₂]
    exact hw₁.bind hw₂

theorem injective_forM {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (hi : Injective d ser) :
    ∀ (l l' : List α) (e : ByteArray), Ser.Writes (l.forM ser) d e → Ser.Writes (l'.forM ser) d e →
      l.length = l'.length → l = l'
  | [], [], _, _, _, _ => rfl
  | x :: l, y :: l', e, hx, hy, hlen => by
    rw [List.forM_cons_ser] at hx hy
    have hu := Ser.uniform_forM (l := l) fun z _ => h.uniform z trivial
    have hu' := Ser.uniform_forM (l := l') fun z _ => h.uniform z trivial
    obtain ⟨ex₁, ex₂, hx₁, hx₂, rfl⟩ := hx.of_bind (h.uniform x trivial) hu
    obtain ⟨ey₁, ey₂, hy₁, hy₂, heq⟩ := hy.of_bind (h.uniform y trivial) hu'
    obtain ⟨rfl, rfl, rfl⟩ := h.injective_of_prefix hi trivial trivial hx₁ hy₁ heq
    rw [injective_forM h hi l l' ex₂ hx₂ hy₂ (by simpa using hlen)]

theorem Array.mkEmpty_append (c : Nat) (xs : Array α) : Array.mkEmpty c ++ xs = xs :=
  Array.empty_append

/-- Elements of an array of known size, as a list. -/
theorem correct_tupleArray_toList {ser : α → Ser Unit} {de : De α} (h : Correct d ser de)
    (n : Nat) : CorrectOn (fun xs : Array α => xs.size = n) d (fun xs => xs.toList.forM ser)
      (De.tupleArray n de) where
  uniform xs _ := Ser.uniform_forM fun y _ => h.uniform y trivial
  decode xs e hxs he input pos hsize hinput := by
    have := decode_elems h xs.toList e (Array.mkEmpty (De.initialCapacity n)) pos he hsize hinput
    rwa [Array.length_toList, hxs] at this
  canonical input pos r pos' hr := by
    obtain ⟨l, rfl, rfl, hle, hw⟩ := canonical_elems h n _ pos r pos' hr
    refine ⟨by simp, hle, ?_⟩
    rw [Array.mkEmpty_append]
    exact hw

/-- Elements of an array of known size. -/
theorem correct_tupleArray {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (n : Nat) :
    CorrectOn (fun xs : Array α => xs.size = n) d (Ser.elems ser) (De.tupleArray n de) :=
  (correct_tupleArray_toList h n).congr (fun xs out => Ser.elems_eq xs d out) (fun _ _ => rfl)

theorem injective_tupleArray {ser : α → Ser Unit} {de : De α} (h : Correct d ser de)
    (hi : Injective d ser) (n : Nat) :
    InjectiveOn (fun xs : Array α => xs.size = n) d (Ser.elems ser) := by
  refine InjectiveOn.congr (ser := fun xs => xs.toList.forM ser) ?_
    (fun xs out => Ser.elems_eq xs d out)
  intro xs ys e hxs hys hx hy
  exact Array.toList_inj.mp (injective_forM h hi _ _ e hx hy (by simp [hxs, hys]))

/-! ## Sequences, fixed-size arrays, and vectors -/

theorem correct_seq {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) {ser : α → Ser Unit} {de : De α}
    (h : Correct d ser de) : Correct d (Ser.seq len ser) (De.seq deLen de) :=
  correct_lengthPrefixed hlen (size := Array.size) (correct_tupleArray h)

theorem injective_seq {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) {ser : α → Ser Unit} {de : De α}
    (h : Correct d ser de) (hi : Injective d ser) : Injective d (Ser.seq len ser) :=
  injective_lengthPrefixed hlen (size := Array.size) (correct_tupleArray h)
    (injective_tupleArray h hi)

theorem correct_fixedArray {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (n : Nat) :
    Correct d (Ser.tupleArray n ser) (De.tupleArray n de) where
  uniform xs _ := by
    unfold Ser.tupleArray
    split
    · exact (correct_tupleArray h n).uniform xs (by assumption)
    · exact Ser.fails_throw.uniform
  decode xs e _ he input pos hsize hinput := by
    unfold Ser.tupleArray at he
    split at he
    · exact (correct_tupleArray h n).decode xs e (by assumption) he input pos hsize hinput
    · exact absurd Ser.fails_throw he.not_fails
  canonical input pos r pos' hr := by
    obtain ⟨hn, hle, hw⟩ := (correct_tupleArray h n).canonical input pos r pos' hr
    refine ⟨trivial, hle, ?_⟩
    unfold Ser.tupleArray
    rw [ite_eq_left hn]
    exact hw

theorem injective_fixedArray {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (hi : Injective d ser) (n : Nat) :
    Injective d (Ser.tupleArray n ser) := by
  intro xs ys e _ _ hx hy
  unfold Ser.tupleArray at hx hy
  split at hx
  · split at hy
    · exact injective_tupleArray h hi n xs ys e (by assumption) (by assumption) hx hy
    · exact absurd Ser.fails_throw hy.not_fails
  · exact absurd Ser.fails_throw hx.not_fails

theorem De.vector_eq_ok {de : De α} :
    De.vector n de ctx pos = .ok y pos' ↔ De.tupleArray n de ctx pos = .ok y.toArray pos' := by
  simp only [De.vector, De.bind_eq_ok]
  constructor
  · rintro ⟨xs, p, hxs, h⟩
    split at h
    · obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
      exact hxs
    · exact absurd h De.throw_ne_ok
  · intro h
    exact ⟨y.toArray, pos', h, by rw [dite_eq_left y.size_toArray]; rfl⟩

theorem correct_vector {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (n : Nat) :
    Correct d (Ser.vector ser) (De.vector n de) where
  uniform xs _ := (correct_tupleArray h n).uniform xs.toArray xs.size_toArray
  decode xs e _ he input pos hsize hinput := by
    obtain ⟨r, hr⟩ := (correct_tupleArray h n).decode xs.toArray e xs.size_toArray he input pos
      hsize hinput
    obtain ⟨hsize', -, -⟩ := (correct_tupleArray h n).canonical input pos r _ hr
    exact ⟨⟨r, hsize'⟩, De.vector_eq_ok.mpr hr⟩
  canonical input pos y pos' hy :=
    let ⟨_, hle, hw⟩ := (correct_tupleArray h n).canonical input pos y.toArray pos'
      (De.vector_eq_ok.mp hy)
    ⟨trivial, hle, hw⟩

theorem injective_vector {ser : α → Ser Unit} {de : De α} (h : Correct d ser de)
    (hi : Injective d ser) (n : Nat) : Injective d (Ser.vector (n := n) ser) :=
  fun xs ys e _ _ hx hy => Vector.toArray_inj.mp
    (injective_tupleArray h hi n xs.toArray ys.toArray e xs.size_toArray ys.size_toArray hx hy)

end Serde
