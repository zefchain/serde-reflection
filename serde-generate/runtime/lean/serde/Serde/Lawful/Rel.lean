/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Std.Data.HashMap.Lemmas

/-!
# Equivalence relations on serialized types

Two hash maps with the same entries may be different Lean values (e.g. if their bucket arrays
have different sizes). For types containing hash maps, the round-trip properties are therefore
stated up to an equivalence relation, which compares hash maps by their contents
(`Serde.Rel.hashMap`), and other values structurally. This file defines the relations used for
each type constructor, and proves that they are equivalences.
-/

namespace Serde

open Std

namespace Rel

theorem eq_equivalence : Equivalence (@Eq α) := ⟨fun _ => rfl, Eq.symm, Eq.trans⟩

theorem true_equivalence : Equivalence fun (_ _ : α) => True := ⟨fun _ => trivial, fun _ => trivial, fun _ _ => trivial⟩

/-- Pairs related componentwise. -/
def prod (r : α → α → Prop) (s : β → β → Prop) (p q : α × β) : Prop := r p.1 q.1 ∧ s p.2 q.2

theorem prod_equivalence (hr : Equivalence r) (hs : Equivalence s) : Equivalence (prod r s) :=
  ⟨fun _ => ⟨hr.refl _, hs.refl _⟩, fun h => ⟨hr.symm h.1, hs.symm h.2⟩,
    fun h h' => ⟨hr.trans h.1 h'.1, hs.trans h.2 h'.2⟩⟩

/-- Lists of the same length, related elementwise. -/
def list (r : α → α → Prop) : List α → List α → Prop
  | [], [] => True
  | x :: xs, y :: ys => r x y ∧ list r xs ys
  | _, _ => False

theorem list_equivalence (hr : Equivalence r) : Equivalence (list r) where
  refl l := by
    induction l with
    | nil => trivial
    | cons x l ih => exact ⟨hr.refl x, ih⟩
  symm {l l'} h := by
    induction l generalizing l' with
    | nil => cases l' <;> simp_all [list]
    | cons x l ih =>
      cases l' with
      | nil => cases h
      | cons y l' => exact ⟨hr.symm h.1, ih h.2⟩
  trans {l l' l''} h h' := by
    induction l generalizing l' l'' with
    | nil => cases l' <;> cases l'' <;> simp_all [list]
    | cons x l ih =>
      cases l' with
      | nil => cases h
      | cons y l' =>
        cases l'' with
        | nil => cases h'
        | cons z l'' => exact ⟨hr.trans h.1 h'.1, ih h.2 h'.2⟩

theorem list_length : ∀ {l l' : List α}, list r l l' → l.length = l'.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by simp [list_length h.2]

/-- Arrays of the same size, related elementwise. -/
def array (r : α → α → Prop) (xs ys : Array α) : Prop := list r xs.toList ys.toList

theorem array_equivalence (hr : Equivalence r) : Equivalence (array r) :=
  ⟨fun _ => (list_equivalence hr).refl _, (list_equivalence hr).symm,
    (list_equivalence hr).trans⟩

/-- Vectors related elementwise. -/
def vector (r : α → α → Prop) (xs ys : Vector α n) : Prop := array r xs.toArray ys.toArray

theorem vector_equivalence (hr : Equivalence r) : Equivalence (vector (n := n) r) :=
  ⟨fun _ => (array_equivalence hr).refl _, (array_equivalence hr).symm,
    (array_equivalence hr).trans⟩

theorem option_equivalence (hr : Equivalence r) : Equivalence (Option.Rel r) where
  refl x := by cases x <;> constructor; exact hr.refl _
  symm h := by cases h <;> constructor; exact hr.symm ‹_›
  trans h h' := by
    cases h <;> cases h' <;> constructor
    exact hr.trans ‹_› ‹_›

theorem sum_equivalence (hr : Equivalence r) (hs : Equivalence s) :
    Equivalence (Sum.LiftRel r s) where
  refl x := by cases x <;> constructor <;> first | exact hr.refl _ | exact hs.refl _
  symm h := by cases h <;> constructor <;> first | exact hr.symm ‹_› | exact hs.symm ‹_›
  trans h h' := by
    cases h <;> cases h' <;> constructor <;>
      first | exact hr.trans ‹_› ‹_› | exact hs.trans ‹_› ‹_›

theorem invImage_equivalence {r : β → β → Prop} (hr : Equivalence r) (f : α → β) :
    Equivalence (InvImage r f) :=
  ⟨fun _ => hr.refl _, hr.symm, hr.trans⟩

/-- Hash maps with the same keys, whose values are related. -/
def hashMap [BEq K] [Hashable K] (r : V → V → Prop) (m₁ m₂ : HashMap K V) : Prop :=
  ∀ k : K, Option.Rel r m₁[k]? m₂[k]?

theorem hashMap_equivalence [BEq K] [Hashable K] (hr : Equivalence r) :
    Equivalence (hashMap (K := K) r) :=
  ⟨fun _ _ => (option_equivalence hr).refl _, fun h k => (option_equivalence hr).symm (h k),
    fun h h' k => (option_equivalence hr).trans (h k) (h' k)⟩

theorem option_rel_eq {a b : Option α} : Option.Rel Eq a b ↔ a = b := by
  constructor
  · intro h; cases h <;> simp_all
  · rintro rfl; exact (option_equivalence eq_equivalence).refl _

/-- With equality on values, `Rel.hashMap` is the equivalence of hash maps. -/
theorem hashMap_eq_iff_equiv [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    {m₁ m₂ : HashMap K V} : hashMap Eq m₁ m₂ ↔ m₁.Equiv m₂ := by
  constructor
  · intro h
    exact HashMap.Equiv.of_forall_getElem?_eq fun k => option_rel_eq.mp (h k)
  · intro h k
    exact option_rel_eq.mpr (HashMap.Equiv.getElem?_eq h)

/-- Arrays of entries with the same keys, whose values are related. -/
def entries (r : V → V → Prop) (xs ys : Array (K × V)) : Prop :=
  (∀ x ∈ xs.toList, ∃ y ∈ ys.toList, x.1 = y.1 ∧ r x.2 y.2) ∧
    (∀ y ∈ ys.toList, ∃ x ∈ xs.toList, x.1 = y.1 ∧ r x.2 y.2)

theorem entries_equivalence (hr : Equivalence r) : Equivalence (entries (K := K) r) where
  refl _ := ⟨fun x hx => ⟨x, hx, rfl, hr.refl _⟩, fun x hx => ⟨x, hx, rfl, hr.refl _⟩⟩
  symm h := ⟨fun y hy => have ⟨x, hx, h₁, h₂⟩ := h.2 y hy; ⟨x, hx, h₁.symm, hr.symm h₂⟩,
    fun x hx => have ⟨y, hy, h₁, h₂⟩ := h.1 x hx; ⟨y, hy, h₁.symm, hr.symm h₂⟩⟩
  trans h h' := ⟨fun x hx =>
      have ⟨y, hy, h₁, h₂⟩ := h.1 x hx
      have ⟨z, hz, h₃, h₄⟩ := h'.1 y hy
      ⟨z, hz, h₁.trans h₃, hr.trans h₂ h₄⟩,
    fun z hz =>
      have ⟨y, hy, h₁, h₂⟩ := h'.2 z hz
      have ⟨x, hx, h₃, h₄⟩ := h.2 y hy
      ⟨x, hx, h₃.trans h₁, hr.trans h₄ h₂⟩⟩

/-! Entries related positionally, with equal keys. -/

theorem list_prod_mem_left {l l' : List (K × V)} (h : list (prod Eq r) l l') (hx : x ∈ l) :
    ∃ y ∈ l', x.1 = y.1 ∧ r x.2 y.2 := by
  induction l generalizing l' with
  | nil => cases hx
  | cons a l ih =>
    cases l' with
    | nil => cases h
    | cons b l' =>
      rcases List.mem_cons.mp hx with rfl | hx
      · exact ⟨b, List.mem_cons_self .., h.1.1, h.1.2⟩
      · obtain ⟨y, hy, h'⟩ := ih h.2 hx
        exact ⟨y, List.mem_cons_of_mem _ hy, h'⟩

theorem list_prod_mem_right {l l' : List (K × V)} (h : list (prod Eq r) l l') (hy : y ∈ l') :
    ∃ x ∈ l, x.1 = y.1 ∧ r x.2 y.2 := by
  induction l generalizing l' with
  | nil => cases l' with
    | nil => cases hy
    | cons => cases h
  | cons a l ih =>
    cases l' with
    | nil => cases h
    | cons b l' =>
      rcases List.mem_cons.mp hy with rfl | hy
      · exact ⟨a, List.mem_cons_self .., h.1.1, h.1.2⟩
      · obtain ⟨x, hx, h'⟩ := ih h.2 hy
        exact ⟨x, List.mem_cons_of_mem _ hx, h'⟩

theorem list_prod_keys {l l' : List (K × V)} (h : list (prod Eq r) l l') :
    l.map Prod.fst = l'.map Prod.fst := by
  induction l generalizing l' with
  | nil => cases l' with
    | nil => rfl
    | cons => cases h
  | cons a l ih =>
    cases l' with
    | nil => cases h
    | cons b l' => simp [h.1.1, ih h.2]

theorem entries_of_array {xs ys : Array (K × V)} (h : array (prod Eq r) xs ys) :
    entries r xs ys :=
  ⟨fun _ hx => list_prod_mem_left h hx, fun _ hy => list_prod_mem_right h hy⟩

/-- A list of entries related to the entries of a hash map gives a related hash map. -/
theorem hashMap_ofList [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    {l : List (K × V)} {m : HashMap K V} (h : list (prod Eq r) l m.toList) :
    hashMap r (HashMap.ofList l) m := by
  have hkeys := list_prod_keys h
  have hl : l.Pairwise fun a b => (a.1 == b.1) = false := by
    have := HashMap.distinct_keys_toList (m := m)
    rw [← List.pairwise_map (R := fun a b => (a == b) = false)] at this ⊢
    rwa [hkeys]
  intro k
  cases hk : m[k]? with
  | none =>
    rw [HashMap.getElem?_ofList_of_contains_eq_false]
    · exact .none
    · rw [hkeys, HashMap.map_fst_toList_eq_keys, HashMap.contains_keys,
        HashMap.contains_eq_isSome_getElem?, hk]
      rfl
  | some v =>
    obtain ⟨x, hx, h₁, h₂⟩ := list_prod_mem_right h (HashMap.mem_toList_iff_getElem?_eq_some.mpr hk)
    obtain ⟨k', v'⟩ := x
    simp only at h₁ h₂
    subst h₁
    rw [HashMap.getElem?_ofList_of_mem (beq_self_eq_true k') hl hx]
    exact .some h₂

end Rel

end Serde
