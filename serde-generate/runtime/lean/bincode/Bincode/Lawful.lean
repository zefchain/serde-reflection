/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bincode.Lawful.Class

/-!
# Proofs of correctness of the Bincode runtime

The specification and the lemmas shared by all formats are in `Serde.Lawful`. This library
proves the parts specific to Bincode: lengths and characters (`Bincode.Lawful.Scalars`), maps
(`Bincode.Lawful.Map`), and the main theorems on `Bincode.toBytes` and `Bincode.fromBytes`
(`Bincode.Lawful.Class`).
-/
