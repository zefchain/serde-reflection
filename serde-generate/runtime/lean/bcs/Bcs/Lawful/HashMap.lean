/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs.Lawful.Map
import Std.Data.HashMap.Lemmas

/-!
# Correctness of hash maps

A hash map is serialized as the array of its entries, sorted by the encodings of their keys,
so its serialization only depends on its contents. Two maps with the same serialization are
related by `Serde.Rel.entries` (for arrays of entries) or `Serde.Rel.hashMap` (for hash maps):
they have the same keys, and related values. With equality on values, `Serde.Rel.hashMap` is
`Std.HashMap.Equiv` (see `Serde.Rel.hashMap_eq_iff_equiv`).
-/

namespace Bcs

open Serde Std

/-! ## Entries up to permutation -/

theorem Chunks.perm {sk : K → Ser Unit} {sv : V → Ser Unit} (h : Chunks sk sv d l cs)
    (hp : l.Perm l') : ∃ cs', Chunks sk sv d l' cs' ∧ cs.Perm cs' := by
  induction hp generalizing cs with
  | nil => cases h; exact ⟨[], .nil, .nil⟩
  | cons x _ ih =>
    cases h with
    | cons hk hv h =>
      obtain ⟨cs', h', hp⟩ := ih h
      exact ⟨_, .cons hk hv h', .cons _ hp⟩
  | swap x y l =>
    cases h with
    | cons hk hv h =>
      cases h with
      | cons hk' hv' h => exact ⟨_, .cons hk' hv' (.cons hk hv h), .swap _ _ _⟩
  | trans _ _ ih₁ ih₂ =>
    obtain ⟨cs₁, h₁, hp₁⟩ := ih₁ h
    obtain ⟨cs₂, h₂, hp₂⟩ := ih₂ h₁
    exact ⟨cs₂, h₂, hp₁.trans hp₂⟩

theorem Chunks.mem_left {sk : K → Ser Unit} {sv : V → Ser Unit} (h : Chunks sk sv d l cs)
    (hx : x ∈ l) : ∃ c ∈ cs, (sk x.1).Writes d c.1 ∧ (sv x.2).Writes d c.2 := by
  induction h with
  | nil => cases hx
  | cons hk hv _ ih =>
    rcases List.mem_cons.mp hx with rfl | hx
    · exact ⟨_, List.mem_cons_self .., hk, hv⟩
    · obtain ⟨c, hc, h⟩ := ih hx
      exact ⟨c, List.mem_cons_of_mem _ hc, h⟩

theorem Chunks.mem_right {sk : K → Ser Unit} {sv : V → Ser Unit} (h : Chunks sk sv d l cs)
    (hc : c ∈ cs) : ∃ x ∈ l, (sk x.1).Writes d c.1 ∧ (sv x.2).Writes d c.2 := by
  induction h with
  | nil => cases hc
  | cons hk hv _ ih =>
    rcases List.mem_cons.mp hc with rfl | hc
    · exact ⟨_, List.mem_cons_self .., hk, hv⟩
    · obtain ⟨x, hx, h⟩ := ih hc
      exact ⟨x, List.mem_cons_of_mem _ hx, h⟩

theorem Chunks.length_eq' {sk : K → Ser Unit} {sv : V → Ser Unit} (h : Chunks sk sv d l cs) :
    cs.length = l.length := h.length_eq.symm

/-- Entries whose key encodings are distinct have distinct keys. -/
theorem Chunks.pairwise_ne {sk : K → Ser Unit} {sv : V → Ser Unit} (h : Chunks sk sv d l cs)
    (hcs : cs.Pairwise fun a b => a.1 ≠ b.1) : l.Pairwise fun a b => a.1 ≠ b.1 := by
  induction h with
  | nil => exact .nil
  | cons hk _ h ih =>
    rw [List.pairwise_cons] at hcs ⊢
    refine ⟨fun x hx heq => ?_, ih hcs.2⟩
    obtain ⟨c', hc', hk', -⟩ := h.mem_left hx
    rw [← heq] at hk'
    exact hcs.1 c' hc' (hk.unique hk')

theorem compareKeys_self (a : ByteArray) : compareKeys a a = .eq :=
  Std.ReflCmp.compare_self (cmp := fun x y : Array UInt8 => compare x y)

theorem keysIncreasing_pairwise_ne : ∀ {p : Option ByteArray} {cs : List (ByteArray × ByteArray)},
    Ser.keysIncreasing p cs → cs.Pairwise fun a b => a.1 ≠ b.1
  | _, [], _ => .nil
  | _, c :: cs, h => by
    simp only [Ser.keysIncreasing, Bool.and_eq_true] at h
    refine .cons (fun c' hc' heq => ?_) (keysIncreasing_pairwise_ne h.2)
    have := keysIncreasing_lt h.2 c' hc'
    rw [← heq, compareKeys_self] at this
    cases this

theorem List.Pairwise.eq_of_fst_eq {l : List (α × β)} (h : l.Pairwise fun a b => a.1 ≠ b.1)
    (ha : a ∈ l) (hb : b ∈ l) (heq : a.1 = b.1) : a = b := by
  induction l with
  | nil => cases ha
  | cons x l ih =>
    rw [List.pairwise_cons] at h
    simp only [List.mem_cons] at ha hb
    rcases ha with rfl | ha
    · rcases hb with rfl | hb
      · rfl
      · exact absurd heq (h.1 b hb)
    · rcases hb with rfl | hb
      · exact absurd heq.symm (h.1 a ha)
      · exact ih h.2 ha hb

theorem entryLE_trans (a b c : ByteArray × ByteArray) :
    entryLE a b = true → entryLE b c = true → entryLE a c = true :=
  Std.TransCmp.isLE_trans (cmp := fun x y : Array UInt8 => compare x y)

theorem entryLE_total (a b : ByteArray × ByteArray) : (entryLE a b || entryLE b a) = true := by
  simp only [entryLE, compareKeys]
  rw [Std.OrientedCmp.eq_swap (cmp := fun x y : Array UInt8 => compare x y) (a := b.1.data)]
  cases compare a.1.data b.1.data <;> rfl

theorem fst_eq_of_entryLE (hab : entryLE a b = true) (hba : entryLE b a = true) : a.1 = b.1 := by
  simp only [entryLE, compareKeys] at hab hba
  rw [Std.OrientedCmp.eq_swap (cmp := fun x y : Array UInt8 => compare x y) (a := b.1.data)] at hba
  have : compare a.1.data b.1.data = .eq := by
    revert hab hba
    cases compare a.1.data b.1.data <;> simp
  exact ByteArray.ext (Std.LawfulEqCmp.compare_eq_iff_eq.mp this)

/-- Sorting permutations of encoded entries with distinct keys gives the same result. -/
theorem mergeSort_eq_of_perm {cs cs' : List (ByteArray × ByteArray)} (hp : cs.Perm cs')
    (hinc : Ser.keysIncreasing none (cs.mergeSort entryLE)) :
    cs.mergeSort entryLE = cs'.mergeSort entryLE := by
  have hp' := (List.mergeSort_perm cs entryLE).trans (hp.trans (List.mergeSort_perm cs' entryLE).symm)
  apply List.Perm.eq_of_pairwise (le := fun a b => entryLE a b = true) _
    (List.pairwise_mergeSort entryLE_trans entryLE_total cs)
    (List.pairwise_mergeSort entryLE_trans entryLE_total cs') hp'
  intro a b ha hb hab hba
  exact List.Pairwise.eq_of_fst_eq (keysIncreasing_pairwise_ne hinc) ha (hp'.mem_iff.mpr hb)
    (fst_eq_of_entryLE hab hba)

theorem Ser.writes_entries_perm {sk : K → Ser Unit} {sv : V → Ser Unit} {xs ys : Array (K × V)}
    (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d)
    (h : (Ser.entries sk sv xs).Writes d e) (hp : xs.toList.Perm ys.toList) :
    (Ser.entries sk sv ys).Writes d e := by
  obtain ⟨cs, hcs, hinc, rfl⟩ := Ser.of_writes_entries hk hv h
  obtain ⟨cs', hcs', hp'⟩ := hcs.perm hp
  have heq := mergeSort_eq_of_perm hp' hinc
  intro out
  rw [Ser.entries_eq hcs', ← heq, ite_eq_left hinc]
  exact Ser.writes_flat _ out

theorem Ser.writes_map_perm {sk : K → Ser Unit} {sv : V → Ser Unit} {xs ys : Array (K × V)}
    (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d)
    (h : (Ser.map sk sv xs).Writes d e) (hp : xs.toList.Perm ys.toList) :
    (Ser.map sk sv ys).Writes d e := by
  have hsize : xs.size = ys.size := by
    rw [← Array.length_toList, ← Array.length_toList, hp.length_eq]
  obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := h.of_bind ((correct_len (d := d)).1.uniform xs.size trivial)
    (Ser.uniform_entries hk hv xs)
  rw [hsize] at h₁
  exact h₁.bind (Ser.writes_entries_perm hk hv h₂ hp)

/-- The keys of a written map have distinct encodings, hence are distinct. -/
theorem Ser.pairwise_ne_of_writes_map {sk : K → Ser Unit} {sv : V → Ser Unit}
    {xs : Array (K × V)} (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d)
    (h : (Ser.map sk sv xs).Writes d e) : xs.toList.Pairwise fun a b => a.1 ≠ b.1 := by
  obtain ⟨e₁, e₂, -, h₂, -⟩ := h.of_bind ((correct_len (d := d)).1.uniform xs.size trivial)
    (Ser.uniform_entries hk hv xs)
  obtain ⟨cs, hcs, hinc, -⟩ := Ser.of_writes_entries hk hv h₂
  refine hcs.pairwise_ne ?_
  exact ((List.mergeSort_perm cs entryLE).pairwise_iff (fun h => Ne.symm h)).mp
    (keysIncreasing_pairwise_ne hinc)

/-! ## Hash maps -/

section

variable [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]

theorem HashMap.toList_ofList_perm {l : List (K × V)} (hl : l.Pairwise fun a b => a.1 ≠ b.1) :
    (HashMap.ofList l).toList.Perm l := by
  have hl' : l.Pairwise fun a b => (a.1 == b.1) = false :=
    hl.imp fun h => beq_eq_false_iff_ne.mpr h
  refine (List.perm_ext_iff_of_nodup ?_ ?_).mpr fun ⟨k, v⟩ => ?_
  · exact List.nodup_iff_pairwise_ne.mpr (HashMap.distinct_keys_toList.imp fun h heq => by
      rw [heq, beq_self_eq_true] at h
      cases h)
  · exact List.nodup_iff_pairwise_ne.mpr (hl.imp fun h heq => h (congrArg Prod.fst heq))
  · rw [HashMap.mem_toList_iff_getElem?_eq_some]
    constructor
    · intro h
      cases hc : (l.map Prod.fst).contains k with
      | false => rw [HashMap.getElem?_ofList_of_contains_eq_false hc] at h; cases h
      | true =>
        obtain ⟨⟨k', v'⟩, hmem, rfl⟩ := List.mem_map.mp (List.contains_iff_mem.mp hc)
        rw [HashMap.getElem?_ofList_of_mem (beq_self_eq_true k') hl' hmem] at h
        cases h
        exact hmem
    · intro h
      exact HashMap.getElem?_ofList_of_mem (beq_self_eq_true k) hl' h

theorem correct_hashMap {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) :
    Correct d (Ser.hashMap sk sv) (De.hashMap dk dv) where
  uniform m _ := (correct_map hk hv).uniform m.toArray trivial
  decode m e _ he input pos hsize hinput := by
    obtain ⟨xs, hxs⟩ := (correct_map hk hv).decode m.toArray e trivial he input pos hsize hinput
    exact ⟨_, De.map_eq_ok.mpr ⟨xs, hxs, rfl⟩⟩
  canonical input pos m pos' h := by
    obtain ⟨xs, hxs, rfl⟩ := De.map_eq_ok.mp h
    obtain ⟨-, hle, hw⟩ := (correct_map hk hv).canonical input pos xs pos' hxs
    have hu : ∀ k, (sk k).Uniform d := fun k => hk.uniform k trivial
    have hu' : ∀ v, (sv v).Uniform d := fun v => hv.uniform v trivial
    refine ⟨trivial, hle, Ser.writes_map_perm hu hu' hw ?_⟩
    simp only [HashMap.toList_toArray]
    exact (HashMap.toList_ofList_perm (Ser.pairwise_ne_of_writes_map hu hu' hw)).symm

end

/-! ## Maps with the same serialization -/

/-- The serialization of an array of map entries: its length, then its sorted entries. -/
theorem Ser.writes_map_chunks {sk : K → Ser Unit} {sv : V → Ser Unit} {xs : Array (K × V)}
    (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d) (h : (Ser.map sk sv xs).Writes d e) :
    ∃ e₁ cs, (Ser.len xs.size).Writes d e₁ ∧ Chunks sk sv d xs.toList cs ∧
      e = e₁ ++ flat (cs.mergeSort entryLE) := by
  obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ := h.of_bind ((correct_len (d := d)).1.uniform xs.size trivial)
    (Ser.uniform_entries hk hv xs)
  obtain ⟨cs, hcs, -, rfl⟩ := Ser.of_writes_entries hk hv h₂
  exact ⟨e₁, cs, h₁, hcs, rfl⟩

/-- Lists of valid encoded entries are determined by their length and concatenation. -/
theorem flat_inj {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) :
    ∀ (cs cs' : List (ByteArray × ByteArray)),
      (∀ c ∈ cs, (∃ k, (sk k).Writes d c.1) ∧ ∃ v, (sv v).Writes d c.2) →
      (∀ c ∈ cs', (∃ k, (sk k).Writes d c.1) ∧ ∃ v, (sv v).Writes d c.2) →
      cs.length = cs'.length → flat cs = flat cs' → cs = cs'
  | [], [], _, _, _, _ => rfl
  | c :: cs, c' :: cs', hc, hc', hlen, heq => by
    obtain ⟨⟨k, hwk⟩, ⟨v, hwv⟩⟩ := hc c (List.mem_cons_self ..)
    obtain ⟨⟨k', hwk'⟩, ⟨v', hwv'⟩⟩ := hc' c' (List.mem_cons_self ..)
    simp only [flat, ByteArray.append_assoc] at heq
    have h₁ := hk.prefix_free trivial trivial hwk hwk' heq
    rw [h₁, ByteArray.append_right_inj] at heq
    have h₂ := hv.prefix_free trivial trivial hwv hwv' heq
    rw [h₂, ByteArray.append_right_inj] at heq
    have := flat_inj hk hv cs cs' (fun c hm => hc c (List.mem_cons_of_mem _ hm))
      (fun c hm => hc' c (List.mem_cons_of_mem _ hm)) (by simpa using hlen) heq
    rw [Prod.ext h₁ h₂, this]

/-- Arrays of entries with the same serialization have the same encoded entries. -/
theorem perm_of_writes_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) {xs ys : Array (K × V)}
    (hx : (Ser.map sk sv xs).Writes d e) (hy : (Ser.map sk sv ys).Writes d e) :
    ∃ cs₁ cs₂, Chunks sk sv d xs.toList cs₁ ∧ Chunks sk sv d ys.toList cs₂ ∧ cs₁.Perm cs₂ := by
  have hl := correct_len (d := d)
  have hku : ∀ k, (sk k).Uniform d := fun k => hk.uniform k trivial
  have hvu : ∀ v, (sv v).Uniform d := fun v => hv.uniform v trivial
  obtain ⟨e₁, cs₁, hl₁, hcs₁, rfl⟩ := Ser.writes_map_chunks hku hvu hx
  obtain ⟨e₂, cs₂, hl₂, hcs₂, heq⟩ := Ser.writes_map_chunks hku hvu hy
  obtain ⟨hsize, rfl, hflat⟩ := hl.1.injective_of_prefix hl.2 trivial trivial hl₁ hl₂ heq
  have valid : ∀ {l : List (K × V)} {cs}, Chunks sk sv d l cs → ∀ c ∈ cs.mergeSort entryLE,
      (∃ k, (sk k).Writes d c.1) ∧ ∃ v, (sv v).Writes d c.2 := by
    intro l cs hcs c hc
    obtain ⟨x, -, h⟩ := hcs.mem_right ((List.mergeSort_perm _ _).mem_iff.mp hc)
    exact ⟨⟨_, h.1⟩, ⟨_, h.2⟩⟩
  have hsorted := flat_inj hk hv _ _ (valid hcs₁) (valid hcs₂) (by
    rw [(List.mergeSort_perm _ _).length_eq, (List.mergeSort_perm _ _).length_eq,
      hcs₁.length_eq', hcs₂.length_eq', Array.length_toList, Array.length_toList, hsize]) hflat
  refine ⟨cs₁, cs₂, hcs₁, hcs₂, ?_⟩
  have p₁ := List.mergeSort_perm cs₁ entryLE
  have p₂ := List.mergeSort_perm cs₂ entryLE
  rw [hsorted] at p₁
  exact p₁.symm.trans p₂

/-- An entry of one list gives an entry of the other one, with the same key. -/
theorem Chunks.transfer {sk : K → Ser Unit} {sv : V → Ser Unit} (ik : Injective d sk)
    (h : Chunks sk sv d l cs) (h' : Chunks sk sv d l' cs') (hp : cs.Perm cs') (hx : x ∈ l) :
    ∃ x' ∈ l', x.1 = x'.1 ∧ ∃ c, (sv x.2).Writes d c ∧ (sv x'.2).Writes d c := by
  obtain ⟨c, hc, hwk, hwv⟩ := h.mem_left hx
  obtain ⟨x', hx', hwk', hwv'⟩ := h'.mem_right (hp.mem_iff.mp hc)
  exact ⟨x', hx', ik _ _ c.1 trivial trivial hwk hwk', c.2, hwv, hwv'⟩


/-- Two arrays of entries with the same serialization have the same keys and related values. -/
theorem injectiveUpTo_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) (ik : Injective d sk)
    (iv : InjectiveUpTo r d sv) : InjectiveUpTo (Rel.entries r) d (Ser.map sk sv) := by
  intro xs ys e hx hy
  obtain ⟨cs₁, cs₂, hcs₁, hcs₂, hp⟩ := perm_of_writes_map hk hv hx hy
  refine ⟨fun x hx' => ?_, fun y hy' => ?_⟩
  · obtain ⟨y, hy, h₁, c, hw, hw'⟩ := hcs₁.transfer ik hcs₂ hp hx'
    exact ⟨y, hy, h₁, iv _ _ c hw hw'⟩
  · obtain ⟨x, hx, h₁, c, hw, hw'⟩ := hcs₂.transfer ik hcs₁ hp.symm hy'
    exact ⟨x, hx, h₁.symm, iv _ _ c hw' hw⟩

/-- Two hash maps with the same serialization have the same keys and related values. -/
theorem injectiveUpTo_hashMap [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) (ik : Injective d sk)
    (iv : InjectiveUpTo r d sv) : InjectiveUpTo (Rel.hashMap r) d (Ser.hashMap sk sv) := by
  intro m₁ m₂ e h₁ h₂
  obtain ⟨cs₁, cs₂, hcs₁, hcs₂, hp⟩ := perm_of_writes_map hk hv h₁ h₂
  rw [HashMap.toList_toArray] at hcs₁ hcs₂
  -- An entry of one map gives an entry of the other one with the same key.
  have transfer : ∀ {m m' : HashMap K V} {cs cs' : List (ByteArray × ByteArray)},
      Chunks sk sv d m.toList cs → Chunks sk sv d m'.toList cs' → cs.Perm cs' →
      ∀ (k : K) (v : V), m[k]? = some v →
        ∃ v' c, m'[k]? = some v' ∧ (sv v).Writes d c ∧ (sv v').Writes d c := by
    intro m m' cs cs' hcs hcs' hp k v hkv
    obtain ⟨⟨k', v'⟩, hx, hk', c, hw, hw'⟩ :=
      hcs.transfer ik hcs' hp (HashMap.mem_toList_iff_getElem?_eq_some.mpr hkv)
    simp only at hk'
    subst hk'
    exact ⟨v', c, HashMap.mem_toList_iff_getElem?_eq_some.mp hx, hw, hw'⟩
  intro k
  cases h : m₁[k]? with
  | none =>
    cases h' : m₂[k]? with
    | none => exact .none
    | some v =>
      obtain ⟨_, _, h'', -⟩ := transfer hcs₂ hcs₁ hp.symm k v h'
      rw [h] at h''
      cases h''
  | some v =>
    obtain ⟨v', c, h', hw, hw'⟩ := transfer hcs₁ hcs₂ hp k v h
    rw [h']
    exact .some (iv v v' c hw hw')

/-! ## Round trips -/

/-- Round trip of maps given as arrays of entries. -/
theorem roundTrip_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (ik : Injective d sk) (hv : Correct d sv dv)
    (rv : RoundTrip r d sv dv) (hr : Equivalence r) :
    RoundTrip (Rel.entries r) d (Ser.map sk sv) (De.map dk dv) :=
  (correct_map hk hv).roundTrip_of_upTo (injectiveUpTo_map hk hv ik (rv.injectiveUpTo hr))

/-- Round trip of hash maps. -/
theorem roundTrip_hashMap [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (ik : Injective d sk) (hv : Correct d sv dv)
    (rv : RoundTrip r d sv dv) (hr : Equivalence r) :
    RoundTrip (Rel.hashMap r) d (Ser.hashMap sk sv) (De.hashMap dk dv) :=
  (correct_hashMap hk hv).roundTrip_of_upTo (injectiveUpTo_hashMap hk hv ik (rv.injectiveUpTo hr))

end Bcs
