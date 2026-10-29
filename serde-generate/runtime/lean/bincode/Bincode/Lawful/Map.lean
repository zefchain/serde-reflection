/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bincode.Lawful.Scalars

/-!
# Correctness of maps

Bincode writes maps as sequences of entries, in the order of the given array, or in the order
of iteration of a hash map. Arrays of entries are therefore (de)serialized like any other
sequence. Hash maps are only decoded up to `Serde.Rel.hashMap`: two hash maps with the same
entries may be different Lean values, and an input may list the entries of a map in any
order (and even repeat keys, in which case the last entry is kept).
-/

namespace Bincode

open Serde Std

theorem correct_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) : Correct d (Ser.map sk sv) (De.map dk dv) :=
  correct_seq (hk.prod hv)

theorem injective_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : Correct d sk dk) (hv : Correct d sv dv) (ik : Injective d sk) (iv : Injective d sv) :
    Injective d (Ser.map sk sv) :=
  injective_seq (hk.prod hv) (Injective.prod hk hv ik iv)

/-- Round trip of maps given as arrays of entries, with values related by `r`. -/
theorem roundTrip_map {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : RoundTrip Eq d sk dk) (hv : RoundTrip r d sv dv) :
    RoundTrip (Rel.entries r) d (Ser.map sk sv) (De.map dk dv) :=
  (roundTrip_seq (hk.prod hv)).mono fun _ _ h => Rel.entries_of_array h

/-- Round trip of hash maps. -/
theorem roundTrip_hashMap [BEq K] [LawfulBEq K] [Hashable K] [LawfulHashable K]
    {sk : K → Ser Unit} {dk : De K} {sv : V → Ser Unit} {dv : De V}
    (hk : RoundTrip Eq d sk dk) (hv : RoundTrip r d sv dv) :
    RoundTrip (Rel.hashMap r) d (Ser.hashMap sk sv) (De.hashMap dk dv) where
  uniform m := (roundTrip_seq (hk.prod hv)).uniform m.toArray
  decode m e he input pos hsize hinput := by
    obtain ⟨xs, hxs, hr⟩ := (roundTrip_seq (hk.prod hv)).decode m.toArray e he input pos hsize hinput
    refine ⟨_, De.map_eq_ok.mpr ⟨xs, hxs, rfl⟩, Rel.hashMap_ofList ?_⟩
    simpa [Rel.array] using hr

end Bincode
