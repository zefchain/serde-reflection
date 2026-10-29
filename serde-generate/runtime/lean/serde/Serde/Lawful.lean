/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Sequences
import Serde.Lawful.Enum
import Serde.Lawful.Rel
import Serde.Lawful.RoundTrip
import Serde.Lawful.Run

/-!
# Proofs of correctness shared by all formats

See `Serde.Lawful.Defs` for the specification. The other modules prove it for the encodings
of `Serde.Basic`: fixed-width integers and other primitive types (`Serde.Lawful.Primitives`
and `Serde.Lawful.Scalars`), options and sequences (`Serde.Lawful.Sequences`), enums and
depth-indexed definitions (`Serde.Lawful.Enum`), and round trips up to the equivalence
relations of `Serde.Lawful.Rel`, for types containing maps (`Serde.Lawful.RoundTrip`).
`Serde.Lawful.Run` relates the specification to `Ser.run` and `De.run`.

Lemmas about sequences and enums are parameterized by the encodings of lengths and variant
indices, which formats provide along with proofs that they are correct and injective.
-/
