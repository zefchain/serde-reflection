/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Defs

/-!
# Generic correctness lemmas

Lemmas on byte slices, and correctness of the generic combinators: mapping, pairs, and
containers (depth limit).
-/

namespace Serde

/-! ## Byte slices -/

theorem extract_split {input e₁ e₂ : ByteArray} {pos : Nat}
    (h : input.extract pos (pos + (e₁ ++ e₂).size) = e₁ ++ e₂) :
    input.extract pos (pos + e₁.size) = e₁ ∧
      input.extract (pos + e₁.size) (pos + e₁.size + e₂.size) = e₂ := by
  rw [ByteArray.size_append] at h
  have h₁ := congrArg (fun b => b.extract 0 e₁.size) h
  have h₂ := congrArg (fun b => b.extract e₁.size (e₁.size + e₂.size)) h
  simp only [ByteArray.extract_extract, ByteArray.extract_append_eq_left rfl,
    ByteArray.extract_append_eq_right rfl rfl, Nat.add_zero] at h₁ h₂
  refine ⟨?_, ?_⟩
  · rwa [Nat.min_eq_left (by omega)] at h₁
  · rwa [Nat.min_eq_left (by omega), ← Nat.add_assoc] at h₂

theorem extract_concat {input : ByteArray} {p₀ p₁ p₂ : Nat} (h₁ : p₀ ≤ p₁) (h₂ : p₁ ≤ p₂) :
    input.extract p₀ p₁ ++ input.extract p₁ p₂ = input.extract p₀ p₂ := by
  rw [ByteArray.extract_append_extract, Nat.min_eq_left h₁, Nat.max_eq_right h₂]

theorem size_extract_le {input : ByteArray} {p q : Nat} : (input.extract p q).size ≤ q - p := by
  rw [ByteArray.size_extract]
  omega

/-! ## Deserializers -/

namespace De

theorem map_eq_ok {f : α → β} {s : De α} :
    (f <$> s) ctx pos = .ok y pos' ↔ ∃ a, s ctx pos = .ok a pos' ∧ f a = y := by
  show (s >>= fun a => pure (f a)) ctx pos = _ ↔ _
  rw [bind_eq_ok]
  simp only [pure_eq_ok]
  constructor
  · rintro ⟨a, p, h, rfl, rfl⟩
    exact ⟨a, h, rfl⟩
  · rintro ⟨a, h, rfl⟩
    exact ⟨a, pos', h, rfl, rfl⟩

theorem throw_ne_ok : (throw e : De α) ctx pos ≠ .ok y pos' := nofun

theorem prod_eq_ok {f : De α} {g : De β} :
    prod f g ctx pos = .ok y pos' ↔
      ∃ p, f ctx pos = .ok y.1 p ∧ g ctx p = .ok y.2 pos' := by
  simp only [prod, bind_eq_ok, pure_eq_ok]
  constructor
  · rintro ⟨a, p, ha, b, p', hb, rfl, rfl⟩
    exact ⟨p, ha, hb⟩
  · rintro ⟨p, ha, hb⟩
    exact ⟨_, p, ha, _, pos', hb, rfl, rfl⟩

end De

/-! ## Prefix-freeness -/

/-- Deserialization is deterministic, hence encodings are prefix-free. -/
theorem CorrectOn.prefix_free (h : CorrectOn P d ser de) (hx : P x) (hy : P y)
    (hex : (ser x).Writes d ex) (hey : (ser y).Writes d ey) (heq : ex ++ r = ey ++ r') :
    ex = ey := by
  obtain ⟨x', hx'⟩ := h.decode x ex hx hex (ex ++ r) 0
    (by simp [ByteArray.size_append]) (by simp [ByteArray.extract_append_eq_left rfl])
  obtain ⟨y', hy'⟩ := h.decode y ey hy hey (ey ++ r') 0
    (by simp [ByteArray.size_append]) (by simp [ByteArray.extract_append_eq_left rfl])
  rw [heq, hy'] at hx'
  simp only [EStateM.Result.ok.injEq] at hx'
  exact ByteArray.append_inj_left heq (by omega)

theorem CorrectOn.injective_of_prefix (h : CorrectOn P d ser de) (hi : InjectiveOn P d ser)
    (hx : P x) (hy : P y) (hex : (ser x).Writes d ex) (hey : (ser y).Writes d ey)
    (heq : ex ++ r = ey ++ r') : x = y ∧ ex = ey ∧ r = r' := by
  have := h.prefix_free hx hy hex hey heq
  subst this
  exact ⟨hi x y ex hx hy hex hey, rfl, (ByteArray.append_right_inj _).mp heq⟩

/-! ## Congruence -/

theorem CorrectOn.congr {ser ser' : α → Ser Unit} {de de' : De α} (h : CorrectOn P d ser de)
    (hser : ∀ x out, ser' x d out = ser x d out)
    (hde : ∀ input pos, de' ⟨input, d⟩ pos = de ⟨input, d⟩ pos) : CorrectOn P d ser' de' where
  uniform x hx := by
    rcases h.uniform x hx with ⟨e, he⟩ | hf
    · exact .inl ⟨e, fun out => (hser x out).trans (he out)⟩
    · exact .inr fun out => by rw [hser]; exact hf out
  decode x e hx he input pos hsize hinput := by
    obtain ⟨y, hy⟩ := h.decode x e hx (fun out => (hser x out).symm.trans (he out)) input pos
      hsize hinput
    exact ⟨y, (hde input pos).trans hy⟩
  canonical input pos y pos' hy := by
    obtain ⟨hP, hle, hw⟩ := h.canonical input pos y pos' ((hde input pos).symm.trans hy)
    exact ⟨hP, hle, fun out => (hser y out).trans (hw out)⟩

theorem InjectiveOn.congr {ser ser' : α → Ser Unit} (h : InjectiveOn P d ser)
    (hser : ∀ x out, ser' x d out = ser x d out) : InjectiveOn P d ser' :=
  fun x y e hx hy hwx hwy => h x y e hx hy (fun out => (hser x out).symm.trans (hwx out))
    (fun out => (hser y out).symm.trans (hwy out))

/-! ## Mapping -/

/--
Converting values before serialization and after deserialization preserves correctness, as
long as converting back and forth is the identity on the deserialized type.
-/
theorem Correct.map {ser : α → Ser Unit} {de : De α} (h : Correct d ser de) (to : β → α)
    (of : α → β) (hto : ∀ a, to (of a) = a) :
    Correct d (fun y => ser (to y)) (of <$> de) where
  uniform y _ := h.uniform (to y) trivial
  decode y e _ he input pos hsize hinput := by
    obtain ⟨a, ha⟩ := h.decode (to y) e trivial he input pos hsize hinput
    exact ⟨of a, De.map_eq_ok.mpr ⟨a, ha, rfl⟩⟩
  canonical input pos y pos' hy := by
    obtain ⟨a, ha, rfl⟩ := De.map_eq_ok.mp hy
    obtain ⟨-, hle, hw⟩ := h.canonical input pos a pos' ha
    exact ⟨trivial, hle, by rwa [hto]⟩

theorem Injective.map {ser : α → Ser Unit} (h : Injective d ser) (to : β → α) (of : α → β)
    (hof : ∀ b, of (to b) = b) : Injective d (fun y => ser (to y)) := by
  intro x y e _ _ hx hy
  rw [← hof x, ← hof y, h (to x) (to y) e trivial trivial hx hy]

/--
Correctness of a serializer that converts values with `to`, and of a deserializer that
converts back with `of?`, failing when the conversion fails. The underlying serializer must
be injective.
-/
theorem CorrectOn.filterMap {ser : α → Ser Unit} {de : De α} (h : CorrectOn P d ser de)
    (hi : InjectiveOn P d ser) (to : β → α) (of? : α → Option β) (hP : ∀ y, P (to y))
    (h₁ : ∀ y, of? (to y) = some y) (h₂ : ∀ a y, of? a = some y → to y = a) {de' : De β}
    (hde : ∀ input pos y pos', de' ⟨input, d⟩ pos = .ok y pos' ↔
      ∃ a, de ⟨input, d⟩ pos = .ok a pos' ∧ of? a = some y) :
    Correct d (fun y => ser (to y)) de' ∧ Injective d (fun y => ser (to y)) := by
  refine ⟨⟨fun y _ => h.uniform (to y) (hP y), ?_, ?_⟩, ?_⟩
  · intro y e _ he input pos hsize hinput
    obtain ⟨a, ha⟩ := h.decode (to y) e (hP y) he input pos hsize hinput
    obtain ⟨hPa, -, hw⟩ := h.canonical input pos a _ ha
    rw [hinput] at hw
    have := hi a (to y) e hPa (hP y) hw he
    subst this
    exact ⟨y, (hde input pos y _).mpr ⟨_, ha, h₁ y⟩⟩
  · intro input pos y pos' hy
    obtain ⟨a, ha, hof⟩ := (hde input pos y pos').mp hy
    obtain ⟨-, hle, hw⟩ := h.canonical input pos a pos' ha
    exact ⟨trivial, hle, by rwa [h₂ a y hof]⟩
  · intro x y e _ _ hx hy
    have := hi (to x) (to y) e (hP x) (hP y) hx hy
    rw [← Option.some_inj, ← h₁ x, ← h₁ y, this]

/-! ## Pairs -/

theorem Correct.prod (ha : Correct d sa da) (hb : Correct d sb db) :
    Correct d (Ser.prod sa sb) (De.prod da db) where
  uniform x _ := (ha.uniform x.1 trivial).bind (hb.uniform x.2 trivial)
  decode x e _ he input pos hsize hinput := by
    obtain ⟨e₁, e₂, h₁, h₂, rfl⟩ :=
      he.of_bind (ha.uniform x.1 trivial) (hb.uniform x.2 trivial)
    obtain ⟨hi₁, hi₂⟩ := extract_split hinput
    rw [ByteArray.size_append] at hsize
    obtain ⟨a, ha'⟩ := ha.decode x.1 e₁ trivial h₁ input pos (by omega) hi₁
    obtain ⟨b, hb'⟩ := hb.decode x.2 e₂ trivial h₂ input (pos + e₁.size) (by omega) hi₂
    refine ⟨(a, b), De.prod_eq_ok.mpr ⟨_, ha', ?_⟩⟩
    rwa [ByteArray.size_append, ← Nat.add_assoc]
  canonical input pos y pos' hy := by
    obtain ⟨p, ha', hb'⟩ := De.prod_eq_ok.mp hy
    obtain ⟨-, hle₁, hw₁⟩ := ha.canonical input pos y.1 p ha'
    obtain ⟨-, hle₂, hw₂⟩ := hb.canonical input p y.2 pos' hb'
    refine ⟨trivial, Nat.le_trans hle₁ hle₂, ?_⟩
    rw [← extract_concat hle₁ hle₂]
    exact hw₁.bind hw₂

theorem Injective.prod (ha : Correct d sa da) (hb : Correct d sb db) (ha' : Injective d sa)
    (hb' : Injective d sb) : Injective d (Ser.prod sa sb) := by
  intro x y e _ _ hx hy
  obtain ⟨ex₁, ex₂, hx₁, hx₂, rfl⟩ :=
    hx.of_bind (ha.uniform x.1 trivial) (hb.uniform x.2 trivial)
  obtain ⟨ey₁, ey₂, hy₁, hy₂, heq⟩ :=
    hy.of_bind (ha.uniform y.1 trivial) (hb.uniform y.2 trivial)
  obtain ⟨h₁, rfl, rfl⟩ := ha.injective_of_prefix ha' trivial trivial hx₁ hy₁ heq
  exact Prod.ext h₁ (hb' x.2 y.2 ex₂ trivial trivial hx₂ hy₂)

/-! ## Containers -/

theorem Ser.container_zero (s : Ser Unit) : (Ser.container s).Fails 0 :=
  fun _ => ⟨_, _, rfl⟩

theorem Ser.writes_container_succ {s : Ser Unit} :
    (Ser.container s).Writes (d + 1) e ↔ s.Writes d e := Iff.rfl

theorem Ser.uniform_container_succ {s : Ser Unit} :
    (Ser.container s).Uniform (d + 1) ↔ s.Uniform d := Iff.rfl

theorem De.container_eq_ok {de : De α} {input : ByteArray} :
    De.container de ⟨input, d⟩ pos = .ok y pos' ↔
      ∃ d', d = d' + 1 ∧ de ⟨input, d'⟩ pos = .ok y pos' := by
  cases d with
  | zero => exact ⟨nofun, fun ⟨_, h, _⟩ => nomatch h⟩
  | succ d => exact ⟨fun h => ⟨d, rfl, h⟩, fun ⟨_, h, h'⟩ => by cases h; exact h'⟩

theorem CorrectOn.container_zero (ser : α → Ser Unit) (de : De α) :
    CorrectOn P 0 (fun x => Ser.container (ser x)) (De.container de) where
  uniform x _ := (Ser.container_zero (ser x)).uniform
  decode x e _ he := absurd (Ser.container_zero (ser x)) he.not_fails
  canonical input pos y pos' h := by
    obtain ⟨_, h, _⟩ := De.container_eq_ok.mp h
    cases h

theorem CorrectOn.container_succ (h : CorrectOn P d ser de) :
    CorrectOn P (d + 1) (fun x => Ser.container (ser x)) (De.container de) where
  uniform x hx := h.uniform x hx
  decode x e hx he input pos hsize hinput := h.decode x e hx he input pos hsize hinput
  canonical input pos y pos' hy := by
    obtain ⟨d', hd, hy⟩ := De.container_eq_ok.mp hy
    cases hd
    exact h.canonical input pos y pos' hy

theorem CorrectOn.container (h : ∀ d, CorrectOn P d ser de) (d : Nat) :
    CorrectOn P d (fun x => Ser.container (ser x)) (De.container de) := by
  cases d with
  | zero => exact container_zero ser de
  | succ d => exact (h d).container_succ

theorem InjectiveOn.container_zero (ser : α → Ser Unit) :
    InjectiveOn P 0 (fun x => Ser.container (ser x)) :=
  fun x _ _ _ _ hx => absurd (Ser.container_zero (ser x)) hx.not_fails

theorem InjectiveOn.container_succ (h : InjectiveOn P d ser) :
    InjectiveOn P (d + 1) (fun x => Ser.container (ser x)) := h

theorem InjectiveOn.container (h : ∀ d, InjectiveOn P d ser) (d : Nat) :
    InjectiveOn P d (fun x => Ser.container (ser x)) := by
  cases d with
  | zero => exact container_zero ser
  | succ d => exact (h d).container_succ

end Serde
