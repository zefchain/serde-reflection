/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/

/-!
Instances needed to use core types as keys of hash maps.
-/

instance [Hashable α] : Hashable (Vector α n) := ⟨fun v => hash v.toArray⟩

instance : LawfulBEq ByteArray where
  rfl {a} := by
    show (a.data == a.data) = true
    simp
  eq_of_beq {a b} h := by
    have : (a.data == b.data) = true := h
    exact ByteArray.ext (by simpa using this)
