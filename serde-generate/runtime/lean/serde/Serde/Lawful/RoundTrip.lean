/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Enum
import Serde.Lawful.Rel

/-!
# Round trips up to equivalence

Lemmas on `RoundTrip r`: the serialization of every value is decoded to a value related to it
by `r`. They are used for types containing hash maps, for which `r` compares hash maps by
their contents (see `Serde.Lawful.Rel`). For other types, `RoundTrip Eq` follows from
correctness and injectivity (`Correct.roundTrip`).
-/

namespace Serde

/-! ## General properties -/

theorem RoundTrip.mono (h : RoundTrip r d ser de) (himp : ∀ x y, r x y → r' x y) :
    RoundTrip r' d ser de where
  uniform := h.uniform
  decode x e he input pos hsize hinput :=
    let ⟨y, hy, hr⟩ := h.decode x e he input pos hsize hinput
    ⟨y, hy, himp y x hr⟩

theorem RoundTrip.congr {ser ser' : α → Ser Unit} {de de' : De α} (h : RoundTrip r d ser de)
    (hser : ∀ x out, ser' x d out = ser x d out)
    (hde : ∀ input pos, de' ⟨input, d⟩ pos = de ⟨input, d⟩ pos) : RoundTrip r d ser' de' where
  uniform x := by
    rcases h.uniform x with ⟨e, he⟩ | hf
    · exact .inl ⟨e, fun out => (hser x out).trans (he out)⟩
    · exact .inr fun out => by rw [hser]; exact hf out
  decode x e he input pos hsize hinput := by
    obtain ⟨y, hy, hr⟩ := h.decode x e (fun out => (hser x out).symm.trans (he out)) input pos
      hsize hinput
    exact ⟨y, (hde input pos).trans hy, hr⟩

/-- A correct serializer that is injective up to `r` round-trips up to `r`. -/
theorem Correct.roundTrip_of_upTo (h : Correct d ser de) (hi : InjectiveUpTo r d ser) :
    RoundTrip r d ser de where
  uniform x := h.uniform x trivial
  decode x e he input pos hsize hinput := by
    obtain ⟨y, hy⟩ := h.decode x e trivial he input pos hsize hinput
    obtain ⟨-, -, hw⟩ := h.canonical input pos y _ hy
    rw [hinput] at hw
    exact ⟨y, hy, hi y x e hw he⟩

/-- A correct and injective serializer round-trips exactly. -/
theorem Correct.roundTrip (h : Correct d ser de) (hi : Injective d ser) : RoundTrip Eq d ser de :=
  h.roundTrip_of_upTo hi.upTo

/-- Values with the same serialization are decoded to the same value, hence are related. -/
theorem RoundTrip.injectiveUpTo (h : RoundTrip r d ser de) (hr : Equivalence r) :
    InjectiveUpTo r d ser := by
  intro x y e hx hy
  have hext : e.extract 0 (0 + e.size) = e := by rw [Nat.zero_add, ByteArray.extract_zero_size]
  obtain ⟨z, hz, hzx⟩ := h.decode x e hx e 0 (by omega) hext
  obtain ⟨z', hz', hzy⟩ := h.decode y e hy e 0 (by omega) hext
  rw [hz] at hz'
  cases hz'
  exact hr.trans (hr.symm hzx) hzy

/-! ## Mapping and pairs -/

theorem RoundTrip.map {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de) (to : β → α)
    (of : α → β) (hto : ∀ a, to (of a) = a) :
    RoundTrip (InvImage r to) d (fun y => ser (to y)) (of <$> de) where
  uniform y := h.uniform (to y)
  decode y e he input pos hsize hinput := by
    obtain ⟨a, ha, hr⟩ := h.decode (to y) e he input pos hsize hinput
    refine ⟨of a, De.map_eq_ok.mpr ⟨a, ha, rfl⟩, ?_⟩
    show r (to (of a)) (to y)
    rwa [hto]

theorem RoundTrip.prod (ha : RoundTrip ra d sa da) (hb : RoundTrip rb d sb db) :
    RoundTrip (Rel.prod ra rb) d (Ser.prod sa sb) (De.prod da db) where
  uniform x := (ha.uniform x.1).bind (hb.uniform x.2)
  decode x e he input pos hsize hinput := by
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := he.of_bind (ha.uniform x.1) (hb.uniform x.2)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨a, ha', hra⟩ := ha.decode x.1 e₁ h₁ input pos (by omega) hi₁
    obtain ⟨b, hb', hrb⟩ := hb.decode x.2 e₂ h₂ input (pos + e₁.size) (by omega) hi₂
    refine ⟨(a, b), De.prod_eq_ok.mpr ⟨_, ha', ?_⟩, hra, hrb⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]

/-! ## Options -/

theorem roundTrip_option {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de) :
    RoundTrip (Option.Rel r) d (Ser.option ser) (De.option de) where
  uniform x := by
    cases x with
    | none => exact (correct_u8 (d := d)).1.uniform 0 trivial
    | some x => exact ((correct_u8 (d := d)).1.uniform 1 trivial).bind (h.uniform x)
  decode x e he input pos hsize hinput := by
    have hu := (correct_u8 (d := d)).1.roundTrip (correct_u8 (d := d)).2
    cases x with
    | none =>
      obtain ⟨b, hb, rfl⟩ := hu.decode 0 e he input pos hsize hinput
      exact ⟨none, De.option_eq_ok.mpr ⟨0, _, hb, .inl ⟨rfl, rfl, rfl⟩⟩, .none⟩
    | some x =>
      obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
        he.of_bind ((correct_u8 (d := d)).1.uniform 1 trivial) (h.uniform x)
      obtain ⟨hi₁, hi₂⟩ := extract_split hinput
      rw [ByteArray.size_append] at hsize
      obtain ⟨b, hb, rfl⟩ := hu.decode 1 e₁ h₁ input pos (by omega) hi₁
      obtain ⟨a, ha, hr⟩ := h.decode x e₂ h₂ input _ (by omega) hi₂
      refine ⟨some a, De.option_eq_ok.mpr ⟨1, _, hb, .inr ⟨rfl, a, ?_, rfl⟩⟩, .some hr⟩
      rwa [ByteArray.size_append, ← Nat.add_assoc]

/-! ## Sequences -/

theorem roundTrip_elems {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de) :
    ∀ (l : List α) (e : ByteArray) (acc : Array α) (pos : Nat), Ser.Writes (l.forM ser) d e →
      pos + e.size ≤ input.size → input.extract pos (pos + e.size) = e →
      ∃ l', De.elems de l.length acc ⟨input, d⟩ pos = .ok (acc ++ l'.toArray) (pos + e.size) ∧
        Rel.list r l' l
  | [], e, acc, pos, he, _, _ => by
    rw [← Ser.writes_forM_nil.unique he]
    exact ⟨[], by simp; rfl, trivial⟩
  | x :: l, e, acc, pos, he, hsize, hinput => by
    rw [List.forM_cons_ser] at he
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := he.of_bind (h.uniform x)
      (Ser.uniform_forM fun y _ => h.uniform y)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨a, ha, hr⟩ := h.decode x e₁ h₁ input pos (by omega) hi₁
    obtain ⟨l', hl', hrl⟩ := roundTrip_elems h l e₂ (acc.push a) (pos + e₁.size) h₂ (by omega) hi₂
    refine ⟨a :: l', De.elems_succ.mpr ⟨a, _, ha, ?_⟩, hr, hrl⟩
    rw [ByteArray.size_append, ← Nat.add_assoc, hl']
    simp

/-- Elements of an array of known size. -/
theorem roundTrip_tupleArray {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de)
    {xs : Array α} (he : (Ser.elems ser xs).Writes d e) (hsize : pos + e.size ≤ input.size)
    (hinput : input.extract pos (pos + e.size) = e) :
    ∃ ys, De.tupleArray xs.size de ⟨input, d⟩ pos = .ok ys (pos + e.size) ∧ Rel.array r ys xs := by
  obtain ⟨l', hl', hr⟩ := roundTrip_elems h xs.toList e (Array.mkEmpty (De.initialCapacity xs.size))
    pos (fun out => (Ser.elems_eq xs d out).symm.trans (he out)) hsize hinput
  rw [Array.length_toList, Array.mkEmpty_append] at hl'
  exact ⟨l'.toArray, hl', by simpa [Rel.array] using hr⟩

theorem Ser.uniform_elems {ser : α → Ser Unit} (h : ∀ x, (ser x).Uniform d) (xs : Array α) :
    (Ser.elems ser xs).Uniform d := by
  rcases Ser.uniform_forM (l := xs.toList) (f := ser) fun y _ => h y with ⟨e, he⟩ | hf
  · exact .inl ⟨e, fun out => (Ser.elems_eq xs d out).trans (he out)⟩
  · exact .inr fun out => by rw [Ser.elems_eq]; exact hf out

theorem roundTrip_seq {len : Nat → Ser Unit} {deLen : De Nat}
    (hlen : Correct d len deLen ∧ Injective d len) {ser : α → Ser Unit} {de : De α}
    (h : RoundTrip r d ser de) :
    RoundTrip (Rel.array r) d (Ser.seq len ser) (De.seq deLen de) where
  uniform xs := (hlen.1.uniform xs.size trivial).bind (Ser.uniform_elems h.uniform xs)
  decode xs e he input pos hsize hinput := by
    have hl := hlen.1.roundTrip hlen.2
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
      he.of_bind (hlen.1.uniform xs.size trivial) (Ser.uniform_elems h.uniform xs)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨n, hn, rfl⟩ := hl.decode xs.size e₁ h₁ input pos (by omega) hi₁
    obtain ⟨ys, hys, hr⟩ := roundTrip_tupleArray h h₂ (by omega) hi₂
    refine ⟨ys, De.bind_eq_ok.mpr ⟨_, _, hn, ?_⟩, hr⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]

theorem roundTrip_fixedArray {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de)
    (n : Nat) : RoundTrip (Rel.array r) d (Ser.tupleArray n ser) (De.tupleArray n de) where
  uniform xs := by
    unfold Ser.tupleArray
    split
    · exact Ser.uniform_elems h.uniform xs
    · exact Ser.fails_throw.uniform
  decode xs e he input pos hsize hinput := by
    unfold Ser.tupleArray at he
    split at he
    · rename_i hn
      subst hn
      exact roundTrip_tupleArray h he hsize hinput
    · exact absurd Ser.fails_throw he.not_fails

theorem roundTrip_vector {ser : α → Ser Unit} {de : De α} (h : RoundTrip r d ser de) (n : Nat) :
    RoundTrip (Rel.vector r) d (Ser.vector ser) (De.vector n de) where
  uniform xs := Ser.uniform_elems h.uniform xs.toArray
  decode xs e he input pos hsize hinput := by
    obtain ⟨ys, hys, hr⟩ := roundTrip_tupleArray h (xs := xs.toArray) he hsize hinput
    have hlen : ys.size = n := by simpa using Rel.list_length hr
    rw [xs.size_toArray] at hys
    exact ⟨⟨ys, hlen⟩, De.vector_eq_ok.mpr hys, hr⟩

/-! ## Containers -/

theorem RoundTrip.container_zero (ser : α → Ser Unit) (de : De α) :
    RoundTrip r 0 (fun x => Ser.container (ser x)) (De.container de) where
  uniform x := (Ser.container_zero (ser x)).uniform
  decode x _ he := absurd (Ser.container_zero (ser x)) he.not_fails

theorem RoundTrip.container_succ (h : RoundTrip r d ser de) :
    RoundTrip r (d + 1) (fun x => Ser.container (ser x)) (De.container de) where
  uniform x := h.uniform x
  decode x e he input pos hsize hinput := h.decode x e he input pos hsize hinput

theorem RoundTrip.container (h : ∀ d, RoundTrip r d ser de) (d : Nat) :
    RoundTrip r d (fun x => Ser.container (ser x)) (De.container de) := by
  cases d with
  | zero => exact container_zero ser de
  | succ d => exact (h d).container_succ

theorem RoundTrip.withDepth {ser : Nat → α → Ser Unit} {de : Nat → De α}
    (h : RoundTrip r d (ser d) (de d)) :
    RoundTrip r d (fun x => Ser.withDepth fun d => ser d x) (De.withDepth de) :=
  h.congr (fun _ _ => rfl) (fun _ _ => rfl)

/-! ## Enums -/

/--
Round trip of an enum, from the fact that the payload of every value is decoded to a related
value. `hdec` is proved by case analysis on values.
-/
theorem roundTrip_enum {tag : UInt32 → Ser Unit} {deTag : De UInt32}
    (hu : Correct d tag deTag ∧ Injective d tag) {index : α → UInt32} {payload : α → Ser Unit}
    {dePayload : Nat → Option (De α)}
    (hdec : ∀ x, ∃ de, dePayload (index x).toNat = some de ∧ (payload x).Uniform d ∧
      ∀ e, (payload x).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
        input.extract pos (pos + e.size) = e →
        ∃ y, de ⟨input, d⟩ pos = .ok y (pos + e.size) ∧ r y x) :
    RoundTrip r d (Ser.enum tag index payload) (De.enum deTag name dePayload) where
  uniform x := by
    obtain ⟨_, _, hux, _⟩ := hdec x
    exact (hu.1.uniform (index x) trivial).bind hux
  decode x e he input pos hsize hinput := by
    have ht := hu.1.roundTrip hu.2
    obtain ⟨de, hde, hux, hdecx⟩ := hdec x
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := he.of_bind (hu.1.uniform (index x) trivial) hux
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨j, hj, rfl⟩ := ht.decode (index x) e₁ h₁ input pos (by omega) hi₁
    obtain ⟨y, hy, hr⟩ := hdecx e₂ h₂ input _ (by omega) hi₂
    refine ⟨y, De.enum_eq_ok.mpr ⟨index x, _, de, hj, hde, ?_⟩, hr⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]

/-- The payload of a variant built with `mk` is decoded to a related value. -/
theorem variant_roundTrip {ser : β → Ser Unit} {de : De β} {rb : β → β → Prop}
    {r : α → α → Prop} (h : RoundTrip rb d ser de) (mk : β → α) (b : β)
    (hmk : ∀ b b', rb b' b → r (mk b') (mk b)) :
    (ser b).Uniform d ∧ ∀ e, (ser b).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
      input.extract pos (pos + e.size) = e →
      ∃ y, (mk <$> de) ⟨input, d⟩ pos = .ok y (pos + e.size) ∧ r y (mk b) := by
  refine ⟨h.uniform b, fun e he input pos hsize hinput => ?_⟩
  obtain ⟨b', hb', hr⟩ := h.decode b e he input pos hsize hinput
  exact ⟨mk b', De.map_eq_ok.mpr ⟨b', hb', rfl⟩, hmk b b' hr⟩

end Serde
