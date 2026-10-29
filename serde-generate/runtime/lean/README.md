# Lean 4 runtimes

Each directory is a standalone [Lake](https://github.com/leanprover/lean4/tree/master/src/lake) package:

* `serde`: definitions shared by all formats: errors, 128-bit integers, the serialization
  monads `Serde.Ser` and `Serde.De`, the encodings common to BCS and Bincode (fixed-width
  integers, options, pairs, fixed-size arrays, ...), and the proof framework (`Serde.Lawful`);
* `bcs`: [BCS](https://github.com/zefchain/bcs) serialization (depends on `serde`);
* `bincode`: [Bincode](https://docs.rs/bincode/1.3.3/bincode/) serialization (depends on `serde`).

The BCS and Bincode runtimes use separate type classes (`Bcs.Serialize`,
`Bincode.Serialize`, ...), so supporting one encoding has no cost for the other. They re-export
the shared encodings under their own names (e.g. `Bcs.Ser.u32` is `Serde.Ser.u32`), and define
the encodings that differ: lengths and variant indices (ULEB128 in BCS, 64-bit and 32-bit
integers in Bincode), maps, characters, and floats (which BCS does not support). As in the
runtimes of the other languages, both formats limit lengths to 2^31 - 1.

They can be used without generated code, e.g.:

```lean
import Bcs

structure Point where
  x : UInt16
  label : Option String
  deriving Inhabited

instance : Bcs.Serialize Point where
  serialize p := Bcs.Ser.container do
    Bcs.serialize p.x
    Bcs.serialize p.label

instance : Bcs.Deserialize Point where
  deserialize := Bcs.De.container do
    return ⟨← Bcs.deserialize, ← Bcs.deserialize⟩

#eval Bcs.toBytes (Point.mk 1 (some "a"))  -- 1 0 1 1 97
```

Only types that have values can be deserialized: `Deserialize` has a field
`nonempty : Nonempty α`, found by instance inference (here, from `deriving Inhabited`). Types
without values, such as `structure S where s : S`, have no `Deserialize` instance. Generated
code defines an `Inhabited` instance for every type, and `serde-generate` rejects registries
containing types without values (external types must have an `Inhabited` instance).

Sequences are `Array`s, fixed-size arrays are `Vector`s, and maps are `Std.HashMap`s. As in
Rust, BCS serializes the entries of a map sorted by the encodings of their keys, and the
deserializer requires strictly increasing keys. Bincode writes the entries in the order of
iteration of the map, and accepts them in any order. Maps can also be handled as arrays of
entries with `Bcs.Ser.map` / `Bcs.De.map` (resp. `Bincode`): generated code does this for maps
inside recursive types (when the values refer to the type being defined, since Lean does not
accept `Std.HashMap` in nested inductive types), and for keys that cannot be hashed.

Both monads track a container-depth budget, consumed by each struct and enum. BCS limits the
depth of containers to 500, as in Rust. Bincode has no limit: its budget (2^62) is larger than
the depth of any value that fits in memory. Recursive types are defined by recursion on this
budget, so no `partial` definitions are needed.

## Proofs of correctness

The modules `Serde.Lawful.*`, `Bcs.Lawful.*`, and `Bincode.Lawful.*` prove that the runtimes
are correct, using only the standard axioms of Lean (`propext`, `Classical.choice`,
`Quot.sound`). The specification (`Serde.Lawful.Defs`) states, for a serializer `ser` and a
deserializer `de`:

* *decoding*: the bytes written by `ser x`, followed by arbitrary data, are accepted by `de`,
  which consumes exactly these bytes;
* *canonicity*: whenever `de` accepts some bytes, they are exactly the serialization of the
  value it returns. In particular, every value has at most one accepted encoding;
* *injectivity*: values with the same serialization are equal.

The lemmas on the shared encodings (`Serde.Lawful`) are proved once, for any encoding of
lengths and variant indices that is itself correct and injective. Each format then proves its
own encodings: ULEB128 integers, sorted maps, and unsupported types for BCS; 64-bit lengths
and UTF-8 characters for Bincode. The main theorems are, for `Bcs` and `Bincode` alike:

```lean
-- Every accepted input is the serialization of the decoded value.
theorem Bcs.toBytes_of_fromBytes [Bcs.LawfulSerialize α] (h : fromBytes b = .ok x) :
    toBytes x = .ok b
-- Every serialization is accepted, and decodes to a value with the same serialization.
theorem Bcs.fromBytes_of_toBytes [Bcs.LawfulSerialize α] (h : toBytes x = .ok b) :
    ∃ y, fromBytes b = .ok y ∧ toBytes y = .ok b
-- For types without maps, encoding and decoding are inverse of each other.
theorem Bcs.fromBytes_eq_ok_iff [Bcs.LawfulSerialize α] [Bcs.InjectiveSerialize α] :
    fromBytes b = .ok x ↔ toBytes x = .ok b
```

Two hash maps with the same entries may be different Lean values (e.g. if their bucket arrays
have different sizes), while they have the same serialization. For types containing maps,
`EquivSerialize` provides an equivalence relation, `Equivalent`, which compares maps by their
contents (keys, and equivalent values) and other values structurally (`Serde.Lawful.Rel`). It
comes with a proof of *round trip*: the serialization of every value decodes to an equivalent
value. The theorems are:

```lean
-- Every serialization is accepted, and decodes to an equivalent value.
theorem Bcs.fromBytes_of_toBytes_equiv [Bcs.EquivSerialize α] (h : toBytes x = .ok b) :
    ∃ y, fromBytes b = .ok y ∧ Bcs.Equivalent y x
-- Values with the same serialization are equivalent.
theorem Bcs.equiv_of_toBytes [Bcs.EquivSerialize α]
    (hx : toBytes x = .ok b) (hy : toBytes y = .ok b) : Bcs.Equivalent x y
-- For hash maps of values without maps, equivalence is `Std.HashMap.Equiv` (`~m`).
theorem Bcs.equiv_hashMap_iff : Bcs.Equivalent m₁ m₂ ↔ m₁.Equiv m₂
```

The two formats differ on maps:

* BCS maps are canonical, so `Bcs.LawfulSerialize` holds for every type, including hash maps:
  an accepted input is always the serialization of the decoded value.
* Bincode accepts the entries of a map in any order (and repeated keys, the last entry being
  kept), so its hash maps are not canonical: `Bincode.LawfulSerialize` only holds for types
  without hash maps, and types with hash maps only have `Bincode.EquivSerialize`.
* Bincode encodes floats with `Float.toBits` and `Float.ofBits`, which are opaque to Lean, so
  types containing floats have no proofs. BCS does not support floats (nor characters):
  serialization and deserialization always fail, which is trivially correct.

Code generated by `serde-generate` comes with instances of `LawfulSerialize`, and of
`InjectiveSerialize` for types without maps or of `EquivSerialize` for types with maps, for
both formats, except for types that depend on external definitions (and for Bincode, types
that contain floats). The equivalence relations of generated types (`T.serdeEquiv`) are shared
by both formats.

Hand-written instances can be proved correct in the same way, e.g. for the type `Point`
above, when the deserializer is written with `Bcs.De.prod`:

```lean
import Bcs.Lawful

instance : Bcs.Deserialize Point where
  deserialize := Bcs.De.container <|
    (fun (b : UInt16 × Option String) => Point.mk b.1 b.2) <$>
      Bcs.De.prod Bcs.deserialize Bcs.deserialize

instance : Bcs.LawfulSerialize Point where
  correct := by
    apply Serde.CorrectOn.container
    intro d
    exact (Bcs.LawfulSerialize.correct d |>.prod (Bcs.LawfulSerialize.correct d)).map
      (fun (p : Point) => (p.x, p.label)) (fun (b : UInt16 × Option String) => Point.mk b.1 b.2)
      (fun _ => rfl)
```

These proofs relate the Lean serializer and deserializer to each other. Compatibility with
other implementations (the Rust crates `bcs` and `bincode`) is tested on sample encodings.

The tests in `tests/` are run by `cargo test --features lean`.
