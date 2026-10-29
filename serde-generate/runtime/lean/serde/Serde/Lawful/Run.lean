/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.RoundTrip

/-!
# Running serializers and deserializers

Consequences of the specification for `Ser.run` and `De.run`, from which formats derive their
theorems on `toBytes` and `fromBytes`.
-/

namespace Serde

theorem Ser.run_eq_ok_iff_writes {s : Ser Unit} (hu : s.Uniform d) :
    s.run d = .ok b ↔ s.Writes d b := by
  constructor
  · intro h
    simp only [Ser.run] at h
    rcases hu with ⟨e, he⟩ | hf
    · rw [he .empty, ByteArray.empty_append] at h
      cases h
      exact he
    · obtain ⟨err, o, hf⟩ := hf .empty
      rw [hf] at h
      cases h
  · intro h
    simp only [Ser.run]
    rw [h .empty, ByteArray.empty_append]

/-- Canonicity: an input accepted by the deserializer is the serialization of its result. -/
theorem Correct.run_canonical (h : Correct d ser de) (hde : de.run d b = .ok x) :
    (ser x).run d = .ok b := by
  simp only [De.run] at hde
  split at hde
  · rename_i pos hde'
    split at hde
    · cases hde
      obtain ⟨-, -, hw⟩ := h.canonical b 0 _ pos hde'
      rw [Ser.run_eq_ok_iff_writes (h.uniform _ trivial)]
      rwa [‹pos = b.size›, ByteArray.extract_zero_size] at hw
    · cases hde
  · cases hde

/-- Every serialization is decoded to a value related to the serialized value. -/
theorem RoundTrip.run (h : RoundTrip r d ser de) (hs : (ser x).run d = .ok b) :
    ∃ y, de.run d b = .ok y ∧ r y x := by
  have hw := (Ser.run_eq_ok_iff_writes (h.uniform x)).mp hs
  obtain ⟨y, hy, hr⟩ := h.decode x b hw b 0 (by omega)
    (by rw [Nat.zero_add, ByteArray.extract_zero_size])
  exact ⟨y, by simp only [De.run, hy, Nat.zero_add, ite_true], hr⟩

/-- Values with the same serialization are related. -/
theorem InjectiveUpTo.run (h : InjectiveUpTo r d ser) (hu : ∀ x, (ser x).Uniform d)
    (hx : (ser x).run d = .ok b) (hy : (ser y).run d = .ok b) : r x y :=
  h x y b ((Ser.run_eq_ok_iff_writes (hu x)).mp hx) ((Ser.run_eq_ok_iff_writes (hu y)).mp hy)

/-- For correct and injective serializers, running them gives inverse functions. -/
theorem Correct.run_eq_ok_iff (h : Correct d ser de) (hi : Injective d ser) :
    de.run d b = .ok x ↔ (ser x).run d = .ok b := by
  refine ⟨h.run_canonical, fun hs => ?_⟩
  obtain ⟨y, hy, rfl⟩ := (h.roundTrip hi).run hs
  exact hy

end Serde
