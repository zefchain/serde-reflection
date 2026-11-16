/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs.Lawful.Scalars

/-!
# Correctness of maps

Maps are arrays of entries. The serializer sorts the entries by the encodings of their keys
and rejects duplicate keys; the deserializer checks that the keys are strictly increasing.
As a consequence, map serialization is not injective (permuting the entries does not change
the encoding), but it is still canonical: an accepted input is always the serialization of the
deserialized map.
-/

namespace Bcs

open Serde

/-- Bytes written for a list of encoded entries. -/
def flat : List (ByteArray × ByteArray) → ByteArray
  | [] => .empty
  | c :: cs => c.1 ++ c.2 ++ flat cs

/-- `cs` are the encodings of the keys and values of the entries `l`. -/
inductive Chunks (sk : K → Ser Unit) (sv : V → Ser Unit) (d : Nat) :
    List (K × V) → List (ByteArray × ByteArray) → Prop
  | nil : Chunks sk sv d [] []
  | cons : (sk e.1).Writes d c.1 → (sv e.2).Writes d c.2 → Chunks sk sv d l cs →
      Chunks sk sv d (e :: l) (c :: cs)

theorem Chunks.length_eq (h : Chunks sk sv d l cs) : l.length = cs.length := by
  induction h with
  | nil => rfl
  | cons _ _ _ ih => simp [ih]

theorem Chunks.mem (h : Chunks sk sv d l cs) (hc : c ∈ cs) :
    (∃ k, (sk k).Writes d c.1) ∧ ∃ v, (sv v).Writes d c.2 := by
  induction h with
  | nil => cases hc
  | cons hk hv _ ih =>
    rcases List.mem_cons.mp hc with rfl | hc
    · exact ⟨⟨_, hk⟩, ⟨_, hv⟩⟩
    · exact ih hc

/-! ## Serialization -/

theorem Ser.capture_of_writes (h : s.Writes d e) : Ser.capture s d out = .ok e out := by
  show (match s d ByteArray.empty with
    | .ok _ b => EStateM.Result.ok b out
    | .error e b => .error e b) = _
  rw [h, ByteArray.empty_append]

theorem Ser.capture_of_fails (h : s.Fails d) : ∃ err o, Ser.capture s d out = .error err o := by
  obtain ⟨err, o, h⟩ := h ByteArray.empty
  refine ⟨err, o, ?_⟩
  show (match s d ByteArray.empty with
    | .ok _ b => EStateM.Result.ok b out
    | .error e b => .error e b) = _
  rw [h]

theorem Ser.mapM_captureEntry_of_chunks (h : Chunks sk sv d l cs) :
    ∀ out, (l.mapM (Ser.captureEntry sk sv)) d out = .ok cs out := by
  induction h with
  | nil => intro out; rfl
  | cons hk hv _ ih =>
    intro out
    rw [List.mapM_cons]
    simp only [Ser.bind_apply, Ser.captureEntry, Ser.capture_of_writes hk,
      Ser.capture_of_writes hv, ih]
    rfl

theorem Ser.mapM_captureEntry_uniform {sk : K → Ser Unit} {sv : V → Ser Unit} :
    ∀ {l : List (K × V)}, (∀ e ∈ l, (sk e.1).Uniform d ∧ (sv e.2).Uniform d) →
      (∃ cs, Chunks sk sv d l cs) ∨
        ∀ out, ∃ err o, (l.mapM (Ser.captureEntry sk sv)) d out = .error err o
  | [], _ => .inl ⟨[], .nil⟩
  | e :: l, hu => by
    obtain ⟨hk, hv⟩ := hu e (List.mem_cons_self ..)
    have ih := Ser.mapM_captureEntry_uniform (l := l) fun e' he' => hu e' (List.mem_cons_of_mem _ he')
    rcases hk with ⟨ck, hk⟩ | hk
    · rcases hv with ⟨cv, hv⟩ | hv
      · rcases ih with ⟨cs, hcs⟩ | hf
        · exact .inl ⟨_, .cons (c := (ck, cv)) hk hv hcs⟩
        · refine .inr fun out => ?_
          obtain ⟨err, o, h⟩ := hf out
          refine ⟨err, o, ?_⟩
          rw [List.mapM_cons]
          simp only [Ser.bind_apply, Ser.captureEntry, Ser.capture_of_writes hk,
            Ser.capture_of_writes hv, Ser.pure_apply, h]
      · refine .inr fun out => ?_
        obtain ⟨err, o, h⟩ := Ser.capture_of_fails (out := out) hv
        refine ⟨err, o, ?_⟩
        rw [List.mapM_cons]
        simp only [Ser.bind_apply, Ser.captureEntry, Ser.capture_of_writes hk, h]
    · refine .inr fun out => ?_
      obtain ⟨err, o, h⟩ := Ser.capture_of_fails (out := out) hk
      refine ⟨err, o, ?_⟩
      rw [List.mapM_cons]
      simp only [Ser.bind_apply, Ser.captureEntry, h]

theorem Ser.writes_flat (cs : List (ByteArray × ByteArray)) :
    Ser.Writes (cs.forM fun c => do Ser.raw c.1; Ser.raw c.2) d (flat cs) := by
  induction cs with
  | nil => exact Ser.writes_forM_nil
  | cons c cs ih =>
    rw [List.forM_cons_ser]
    exact (Ser.writes_raw.bind Ser.writes_raw).bind ih

/-- Order used to sort entries. -/
abbrev entryLE (a b : ByteArray × ByteArray) : Bool := (compareKeys a.1 b.1).isLE

theorem Ser.entries_eq {sk : K → Ser Unit} {sv : V → Ser Unit} {xs : Array (K × V)}
    (h : Chunks sk sv d xs.toList cs) :
    Ser.entries sk sv xs d out =
      if Ser.keysIncreasing none (cs.mergeSort entryLE) then
        ((cs.mergeSort entryLE).forM (fun c => do Ser.raw c.1; Ser.raw c.2) : Ser Unit) d out
      else .error .invalidMapOrder out := by
  simp only [Ser.entries, Ser.bind_apply, Ser.mapM_captureEntry_of_chunks h]
  split <;> rfl

theorem Ser.uniform_entries {sk : K → Ser Unit} {sv : V → Ser Unit} (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d)
    (xs : Array (K × V)) : (Ser.entries sk sv xs).Uniform d := by
  rcases Ser.mapM_captureEntry_uniform (sk := sk) (sv := sv) (l := xs.toList)
    (fun e _ => ⟨hk e.1, hv e.2⟩) with ⟨cs, hcs⟩ | hf
  · by_cases hinc : Ser.keysIncreasing none (cs.mergeSort entryLE)
    · refine .inl ⟨flat (cs.mergeSort entryLE), fun out => ?_⟩
      rw [Ser.entries_eq hcs, ite_eq_left hinc]
      exact Ser.writes_flat _ out
    · refine .inr fun out => ⟨.invalidMapOrder, out, ?_⟩
      rw [Ser.entries_eq hcs, ite_eq_right hinc]
  · refine .inr fun out => ?_
    obtain ⟨err, o, h⟩ := hf out
    exact ⟨err, o, by simp only [Ser.entries, Ser.bind_apply, h]⟩

theorem Ser.of_writes_entries {sk : K → Ser Unit} {sv : V → Ser Unit} {xs : Array (K × V)} (hk : ∀ k, (sk k).Uniform d) (hv : ∀ v, (sv v).Uniform d)
    (h : (Ser.entries sk sv xs).Writes d e) :
    ∃ cs, Chunks sk sv d xs.toList cs ∧ Ser.keysIncreasing none (cs.mergeSort entryLE) ∧
      e = flat (cs.mergeSort entryLE) := by
  rcases Ser.mapM_captureEntry_uniform (sk := sk) (sv := sv) (l := xs.toList)
    (fun e _ => ⟨hk e.1, hv e.2⟩) with ⟨cs, hcs⟩ | hf
  · have he := h .empty
    rw [Ser.entries_eq hcs] at he
    split at he
    · refine ⟨cs, hcs, by assumption, ?_⟩
      rw [Ser.writes_flat _ .empty] at he
      simp only [EStateM.Result.ok.injEq, ByteArray.empty_append] at he
      exact he.2.symm
    · cases he
  · obtain ⟨err, o, hf⟩ := hf .empty
    have he := h .empty
    simp only [Ser.entries, Ser.bind_apply, hf] at he
    cases he

/-! ## Sorting -/

theorem compareKeys_trans (h₁ : compareKeys a b = .lt) (h₂ : compareKeys b c = .lt) :
    compareKeys a c = .lt :=
  Std.TransCmp.lt_trans (cmp := fun x y : Array UInt8 => compare x y) h₁ h₂

theorem keysIncreasing_lt : ∀ {p : ByteArray} {cs : List (ByteArray × ByteArray)},
    Ser.keysIncreasing (some p) cs → ∀ c ∈ cs, compareKeys p c.1 = .lt
  | _, [], _, _, hc => by cases hc
  | p, c :: cs, h, c', hc' => by
    simp only [Ser.keysIncreasing, Option.all_some, Bool.and_eq_true, beq_iff_eq] at h
    rcases List.mem_cons.mp hc' with rfl | hc'
    · exact h.1
    · exact compareKeys_trans h.1 (keysIncreasing_lt h.2 c' hc')

theorem keysIncreasing_pairwise : ∀ {p : Option ByteArray} {cs : List (ByteArray × ByteArray)},
    Ser.keysIncreasing p cs → cs.Pairwise fun a b => entryLE a b = true
  | _, [], _ => .nil
  | _, c :: cs, h => by
    simp only [Ser.keysIncreasing, Bool.and_eq_true] at h
    refine .cons (fun c' hc' => ?_) (keysIncreasing_pairwise h.2)
    simp [entryLE, keysIncreasing_lt h.2 c' hc']

theorem mergeSort_of_keysIncreasing (h : Ser.keysIncreasing p cs) : cs.mergeSort entryLE = cs :=
  List.mergeSort_of_pairwise (keysIncreasing_pairwise h)

/-! ## Deserialization -/

theorem De.entries_succ_eq_ok {dk : De K} {dv : De V} :
    De.entries dk dv (n + 1) prev acc ctx pos = .ok r pos' ↔
      ∃ k p₁ v p₂, dk ctx pos = .ok k p₁ ∧
        prev.all (compareKeys · (ctx.input.extract pos p₁) == .lt) ∧
        dv ctx p₁ = .ok v p₂ ∧
        De.entries dk dv n (some (ctx.input.extract pos p₁)) (acc.push (k, v)) ctx p₂ =
          .ok r pos' := by
  simp only [De.entries, De.bind_eq_ok, De.position_eq_ok, De.input_eq_ok]
  constructor
  · rintro ⟨s, p₀, ⟨hs, hp₀⟩, k, p₁, hk, i, p₁', ⟨hi, hp₁'⟩, q, p₁'', ⟨hq, hp₁''⟩, h⟩
    subst hs hp₀ hi hp₁' hq hp₁''
    split at h
    · obtain ⟨v, p₂, hv, h⟩ := De.bind_eq_ok.mp h
      exact ⟨k, _, v, p₂, hk, by assumption, hv, h⟩
    · exact absurd h De.throw_ne_ok
  · rintro ⟨k, p₁, v, p₂, hk, hprev, hv, h⟩
    refine ⟨pos, pos, ⟨rfl, rfl⟩, k, p₁, hk, ctx.input, p₁, ⟨rfl, rfl⟩, p₁, p₁, ⟨rfl, rfl⟩, ?_⟩
    rw [ite_eq_left hprev]
    exact De.bind_eq_ok.mpr ⟨v, p₂, hv, h⟩

theorem decode_entries {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    {input : ByteArray} (hk : Correct d sk dk) (hv : Correct d sv dv)
    (cs : List (ByteArray × ByteArray)) (prev : Option ByteArray) (acc : Array (K × V))
    (pos : Nat) (hcs : ∀ c ∈ cs, (∃ k, (sk k).Writes d c.1) ∧ ∃ v, (sv v).Writes d c.2)
    (hinc : Ser.keysIncreasing prev cs) (hsize : pos + (flat cs).size ≤ input.size)
    (hinput : input.extract pos (pos + (flat cs).size) = flat cs) :
    ∃ r, De.entries dk dv cs.length prev acc ⟨input, d⟩ pos = .ok r (pos + (flat cs).size) := by
  induction cs generalizing prev acc pos with
  | nil => exact ⟨acc, by simp [flat, ByteArray.size_empty]; rfl⟩
  | cons c cs ih =>
    obtain ⟨⟨k, hwk⟩, ⟨v, hwv⟩⟩ := hcs c (List.mem_cons_self ..)
    have hflat : flat (c :: cs) = (c.1 ++ c.2) ++ flat cs := rfl
    rw [hflat] at hsize hinput ⊢
    obtain ⟨hi₁₂, hi₃⟩ := extract_split hinput
    obtain ⟨hi₁, hi₂⟩ := extract_split hi₁₂
    simp only [ByteArray.size_append] at hsize hi₃ ⊢
    obtain ⟨k', hk'⟩ := hk.decode k c.1 trivial hwk input pos (by omega) hi₁
    obtain ⟨v', hv'⟩ := hv.decode v c.2 trivial hwv input _ (by omega) hi₂
    simp only [Ser.keysIncreasing, Bool.and_eq_true] at hinc
    rw [← Nat.add_assoc] at hi₃
    obtain ⟨r, hr⟩ := ih (some c.1) (acc.push (k', v')) (pos + c.1.size + c.2.size)
      (fun c' hc' => hcs c' (List.mem_cons_of_mem _ hc')) hinc.2 (by omega) hi₃
    refine ⟨r, De.entries_succ_eq_ok.mpr ⟨k', _, v', _, hk', ?_, hv', ?_⟩⟩
    · rw [hi₁]
      exact hinc.1
    · rw [hi₁, show pos + (c.1.size + c.2.size + (flat cs).size) =
        pos + c.1.size + c.2.size + (flat cs).size by omega]
      exact hr

theorem canonical_entries {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) :
    ∀ (n : Nat) (prev : Option ByteArray) (acc : Array (K × V)) (pos : Nat) (r : Array (K × V))
      (pos' : Nat), De.entries dk dv n prev acc ⟨input, d⟩ pos = .ok r pos' →
      ∃ l cs, r = acc ++ l.toArray ∧ l.length = n ∧ Chunks sk sv d l cs ∧
        Ser.keysIncreasing prev cs ∧ pos ≤ pos' ∧ input.extract pos pos' = flat cs
  | 0, _, acc, pos, r, pos', h => by
    obtain ⟨rfl, rfl⟩ := De.pure_eq_ok.mp h
    exact ⟨[], [], by simp, rfl, .nil, rfl, Nat.le_refl _, ByteArray.extract_same⟩
  | n + 1, prev, acc, pos, r, pos', h => by
    obtain ⟨k, p₁, v, p₂, hk', hprev, hv', h⟩ := De.entries_succ_eq_ok.mp h
    obtain ⟨-, hle₁, hwk⟩ := hk.canonical input pos k p₁ hk'
    obtain ⟨-, hle₂, hwv⟩ := hv.canonical input p₁ v p₂ hv'
    obtain ⟨l, cs, rfl, rfl, hcs, hinc, hle₃, hext⟩ :=
      canonical_entries hk hv n _ (acc.push (k, v)) p₂ r pos' h
    refine ⟨(k, v) :: l, (input.extract pos p₁, input.extract p₁ p₂) :: cs, by simp, rfl,
      .cons hwk hwv hcs, ?_, by omega, ?_⟩
    · simp only [Ser.keysIncreasing, Bool.and_eq_true]
      exact ⟨hprev, hinc⟩
    · simp only [flat]
      rw [← hext, extract_concat hle₁ hle₂, extract_concat (by omega) hle₃]

/-! ## Maps -/

theorem correct_entries {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) (n : Nat) :
    CorrectOn (fun xs : Array (K × V) => xs.size = n) d (Ser.entries sk sv)
      (De.entries dk dv n none (Array.mkEmpty (De.initialCapacity n))) where
  uniform xs _ :=
    Ser.uniform_entries (fun k => hk.uniform k trivial) (fun v => hv.uniform v trivial) xs
  decode xs e hxs he input pos hsize hinput := by
    obtain ⟨cs, hcs, hinc, rfl⟩ := Ser.of_writes_entries (fun k => hk.uniform k trivial)
      (fun v => hv.uniform v trivial) he
    have hlen : (cs.mergeSort entryLE).length = n := by
      rw [(List.mergeSort_perm cs entryLE).length_eq, ← hcs.length_eq, Array.length_toList, hxs]
    have := decode_entries hk hv (cs.mergeSort entryLE) none
      (Array.mkEmpty (De.initialCapacity n)) pos
      (fun c hc => hcs.mem ((List.mergeSort_perm cs entryLE).mem_iff.mp hc)) hinc hsize hinput
    rwa [hlen] at this
  canonical input pos r pos' h := by
    obtain ⟨l, cs, rfl, rfl, hcs, hinc, hle, hext⟩ := canonical_entries hk hv _ none _ pos r pos' h
    rw [Array.mkEmpty_append]
    refine ⟨by simp, hle, fun out => ?_⟩
    have hcs' : Chunks sk sv d l.toArray.toList cs := by simpa using hcs
    rw [Ser.entries_eq hcs', mergeSort_of_keysIncreasing hinc, ite_eq_left hinc, hext]
    exact Ser.writes_flat cs out

theorem correct_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) : Correct d (Ser.map sk sv) (De.map dk dv) :=
  correct_lengthPrefixed correct_len (size := Array.size) (correct_entries hk hv)

end Bcs
