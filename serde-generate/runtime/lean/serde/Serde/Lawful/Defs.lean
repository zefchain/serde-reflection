/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Basic

/-!
# Specification of serializers and deserializers

A serializer `ser : α → Ser Unit` and a deserializer `de : De α` are *correct* (at a given
depth budget) when:

* serializing never depends on what was written before (`Ser.Uniform`);
* the bytes written for `x`, followed by arbitrary data, are decoded successfully, and the
  decoder consumes exactly these bytes (`CorrectOn.decode`);
* whenever the decoder succeeds on some input, the bytes it consumed are exactly the
  serialization of the decoded value (`CorrectOn.canonical`). In particular, every value has
  at most one accepted encoding.

The serializer is *injective* when two values are never serialized to the same bytes.
Together, these properties give `fromBytes b = .ok x ↔ toBytes x = .ok b`.

For types containing hash maps, distinct values may have the same contents, and formats may
accept several encodings of the same map. The weaker property `RoundTrip r` states that the
serialization of every value is decoded to a value related to it by `r`, an equivalence
relation that compares hash maps by their contents.
-/

namespace Serde

/-- `s` succeeds and appends `e` to the output, whatever the output is. -/
def Ser.Writes (s : Ser Unit) (d : Nat) (e : ByteArray) : Prop :=
  ∀ out, s d out = .ok () (out ++ e)

/-- `s` fails, whatever the output is. -/
def Ser.Fails (s : Ser Unit) (d : Nat) : Prop :=
  ∀ out, ∃ err out', s d out = .error err out'

/-- `s` either always appends the same bytes or always fails. -/
def Ser.Uniform (s : Ser Unit) (d : Nat) : Prop :=
  (∃ e, s.Writes d e) ∨ s.Fails d

/--
Correctness of a serializer and a deserializer with the depth budget `d`, restricted to
values satisfying `P`. The deserializer only returns values satisfying `P`.
-/
structure CorrectOn (P : α → Prop) (d : Nat) (ser : α → Ser Unit) (de : De α) : Prop where
  uniform : ∀ x, P x → (ser x).Uniform d
  decode : ∀ x e, P x → (ser x).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
    input.extract pos (pos + e.size) = e → ∃ y, de ⟨input, d⟩ pos = .ok y (pos + e.size)
  canonical : ∀ input pos y pos', de ⟨input, d⟩ pos = .ok y pos' →
    P y ∧ pos ≤ pos' ∧ (ser y).Writes d (input.extract pos pos')

/-- Correctness of a serializer and a deserializer with the depth budget `d`. -/
abbrev Correct (d : Nat) (ser : α → Ser Unit) (de : De α) : Prop :=
  CorrectOn (fun _ => True) d ser de

/-- Distinct values satisfying `P` have distinct serializations. -/
def InjectiveOn (P : α → Prop) (d : Nat) (ser : α → Ser Unit) : Prop :=
  ∀ x y e, P x → P y → (ser x).Writes d e → (ser y).Writes d e → x = y

/-- Distinct values have distinct serializations. -/
abbrev Injective (d : Nat) (ser : α → Ser Unit) : Prop :=
  InjectiveOn (fun _ => True) d ser

/-- Values with the same serialization are related by `r`. -/
def InjectiveUpTo (r : α → α → Prop) (d : Nat) (ser : α → Ser Unit) : Prop :=
  ∀ x y e, (ser x).Writes d e → (ser y).Writes d e → r x y

theorem Injective.upTo (h : Injective d ser) : InjectiveUpTo Eq d ser :=
  fun x y e hx hy => h x y e trivial trivial hx hy

/--
The serialization of every value, followed by arbitrary data, is decoded to a value related
to it by `r` (with the decoded value on the left), and the decoder consumes exactly these
bytes.
-/
structure RoundTrip (r : α → α → Prop) (d : Nat) (ser : α → Ser Unit) (de : De α) : Prop where
  uniform : ∀ x, (ser x).Uniform d
  decode : ∀ x e, (ser x).Writes d e → ∀ input pos, pos + e.size ≤ input.size →
    input.extract pos (pos + e.size) = e →
    ∃ y, de ⟨input, d⟩ pos = .ok y (pos + e.size) ∧ r y x

/-! ## Monad laws -/

theorem Ser.ext {s t : Ser α} (h : ∀ d out, s d out = t d out) : s = t := by
  cases s; cases t
  congr
  funext d out
  exact h d out

theorem De.ext {s t : De α} (h : ∀ ctx pos, s ctx pos = t ctx pos) : s = t := by
  cases s; cases t
  congr
  funext ctx pos
  exact h ctx pos

instance : LawfulMonad Ser := LawfulMonad.mk' Ser
  (id_map := fun x => by
    apply Ser.ext
    intro d out
    show (match x d out with
      | .ok a out => EStateM.Result.ok (id a) out
      | .error e out => .error e out) = _
    split <;> simp_all)
  (pure_bind := fun _ _ => rfl)
  (bind_assoc := fun x f g => by
    apply Ser.ext
    intro d out
    show (match (match x d out with
      | .ok a out => f a d out
      | .error e out => .error e out) with
      | .ok a out => g a d out
      | .error e out => .error e out) = (match x d out with
      | .ok a out => (match f a d out with
        | .ok a out => g a d out
        | .error e out => .error e out)
      | .error e out => .error e out)
    cases x d out <;> rfl)

instance : LawfulMonad De := LawfulMonad.mk' De
  (id_map := fun x => by
    apply De.ext
    intro ctx pos
    show (match x ctx pos with
      | .ok a pos => EStateM.Result.ok (id a) pos
      | .error e pos => .error e pos) = _
    split <;> simp_all)
  (pure_bind := fun _ _ => rfl)
  (bind_assoc := fun x f g => by
    apply De.ext
    intro ctx pos
    show (match (match x ctx pos with
      | .ok a pos => f a ctx pos
      | .error e pos => .error e pos) with
      | .ok a pos => g a ctx pos
      | .error e pos => .error e pos) = (match x ctx pos with
      | .ok a pos => (match f a ctx pos with
        | .ok a pos => g a ctx pos
        | .error e pos => .error e pos)
      | .error e pos => .error e pos)
    cases x ctx pos <;> rfl)

/-! ## Running serializers -/

namespace Ser

theorem bind_apply (s : Ser α) (k : α → Ser β) (d : Nat) (out : ByteArray) :
    (s >>= k) d out = match s d out with
      | .ok a out' => k a d out'
      | .error e out' => .error e out' := rfl

@[simp] theorem pure_apply (a : α) (d : Nat) (out : ByteArray) :
    (pure a : Ser α) d out = .ok a out := rfl

theorem Writes.unique (h₁ : Writes s d e₁) (h₂ : Writes s d e₂) : e₁ = e₂ := by
  have := (h₁ .empty).symm.trans (h₂ .empty)
  simp only [EStateM.Result.ok.injEq, ByteArray.empty_append] at this
  exact this.2

theorem Writes.not_fails (h : Writes s d e) : ¬ Fails s d := by
  intro hf
  obtain ⟨err, out', h'⟩ := hf .empty
  simp only [h .empty, reduceCtorEq] at h'

theorem Writes.uniform (h : Writes s d e) : Uniform s d := .inl ⟨e, h⟩

theorem Fails.uniform (h : Fails s d) : Uniform s d := .inr h

theorem writes_pure (d : Nat) : Writes (pure () : Ser Unit) d .empty := by
  intro out
  rw [ByteArray.append_empty]
  rfl

theorem Writes.bind {s₁ s₂ : Ser Unit} (h₁ : Writes s₁ d e₁) (h₂ : Writes s₂ d e₂) :
    Writes (do s₁; s₂) d (e₁ ++ e₂) := by
  intro out
  simp only [bind_apply, h₁ out, h₂ (out ++ e₁), ByteArray.append_assoc]

theorem Fails.bind_left {s₁ : Ser Unit} {k : Unit → Ser Unit} (h : Fails s₁ d) :
    Fails (s₁ >>= k) d := by
  intro out
  obtain ⟨err, out', h'⟩ := h out
  exact ⟨err, out', by simp only [bind_apply, h']⟩

theorem Fails.bind_right {s₁ s₂ : Ser Unit} (h₁ : Writes s₁ d e₁) (h : Fails s₂ d) :
    Fails (do s₁; s₂) d := by
  intro out
  obtain ⟨err, out', h'⟩ := h (out ++ e₁)
  exact ⟨err, out', by simp only [bind_apply, h₁ out, h']⟩

theorem Uniform.bind {s₁ s₂ : Ser Unit} (h₁ : Uniform s₁ d) (h₂ : Uniform s₂ d) :
    Uniform (do s₁; s₂) d := by
  rcases h₁ with ⟨e₁, h₁⟩ | h₁
  · rcases h₂ with ⟨e₂, h₂⟩ | h₂
    · exact (h₁.bind h₂).uniform
    · exact (Fails.bind_right h₁ h₂).uniform
  · exact h₁.bind_left.uniform

/-- Decomposes the bytes written by a sequence of two uniform serializers. -/
theorem Writes.of_bind {s₁ s₂ : Ser Unit} (h₁ : Uniform s₁ d) (h₂ : Uniform s₂ d)
    (h : Writes (do s₁; s₂) d e) : ∃ e₁ e₂, Writes s₁ d e₁ ∧ Writes s₂ d e₂ ∧ e = e₁ ++ e₂ := by
  rcases h₁ with ⟨e₁, h₁⟩ | h₁
  · rcases h₂ with ⟨e₂, h₂⟩ | h₂
    · exact ⟨e₁, e₂, h₁, h₂, h.unique (h₁.bind h₂)⟩
    · exact absurd (Fails.bind_right h₁ h₂) h.not_fails
  · exact absurd h₁.bind_left h.not_fails

end Ser

/-! ## Running deserializers -/

namespace De

theorem bind_apply (s : De α) (k : α → De β) (ctx : DeContext) (pos : Nat) :
    (s >>= k) ctx pos = match s ctx pos with
      | .ok a pos' => k a ctx pos'
      | .error e pos' => .error e pos' := rfl

theorem bind_eq_ok {da : De α} {k : α → De β} :
    (da >>= k) ctx pos = .ok y pos' ↔ ∃ a p, da ctx pos = .ok a p ∧ k a ctx p = .ok y pos' := by
  rw [bind_apply]
  constructor
  · intro h
    split at h
    · exact ⟨_, _, by assumption, h⟩
    · cases h
  · rintro ⟨a, p, h₁, h₂⟩
    rw [h₁]
    exact h₂

@[simp] theorem pure_apply (a : α) (ctx : DeContext) (pos : Nat) :
    (pure a : De α) ctx pos = .ok a pos := rfl

theorem pure_eq_ok {a : α} : (pure a : De α) ctx pos = .ok y pos' ↔ a = y ∧ pos = pos' := by
  show EStateM.Result.ok a pos = _ ↔ _
  simp

end De

end Serde
