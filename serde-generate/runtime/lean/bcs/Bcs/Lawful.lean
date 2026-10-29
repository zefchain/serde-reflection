/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs.Lawful.Class

/-!
# Proofs of correctness of the BCS runtime

The specification and the lemmas shared by all formats are in `Serde.Lawful`. This library
proves the parts specific to BCS: ULEB128 integers (`Bcs.Lawful.Uleb128`), lengths and
unsupported types (`Bcs.Lawful.Scalars`), maps (`Bcs.Lawful.Map` and `Bcs.Lawful.HashMap`),
and the main theorems on `Bcs.toBytes` and `Bcs.fromBytes` (`Bcs.Lawful.Class`).
-/
