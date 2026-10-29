/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Bcs
import Bcs.Lawful
import Bincode
import Bincode.Lawful

/-! Unit tests for the BCS and Bincode runtimes, used without generated code. -/

open Serde

/-- A hand-written type with BCS support. -/
structure Point where
  x : UInt16
  label : Option String
  deriving Repr, BEq, Inhabited

instance : Bcs.Serialize Point where
  serialize p := Bcs.Ser.container do
    Bcs.serialize p.x
    Bcs.serialize p.label

instance : Bcs.Deserialize Point where
  deserialize := Bcs.De.container do
    return ⟨← Bcs.deserialize, ← Bcs.deserialize⟩

/-! The proofs of the runtimes only depend on the standard axioms. -/

/-- info: 'Bcs.fromBytes_eq_ok_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms Bcs.fromBytes_eq_ok_iff

/-- info: 'Bcs.correct_map' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms Bcs.correct_map

/-- info: 'Bincode.fromBytes_eq_ok_iff' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms Bincode.fromBytes_eq_ok_iff

/-- info: 'Bincode.roundTrip_hashMap' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms Bincode.roundTrip_hashMap

example (b : ByteArray) (x : Array (Option (String × UInt64))) :
    Bcs.fromBytes b = .ok x ↔ Bcs.toBytes x = .ok b :=
  Bcs.fromBytes_eq_ok_iff

example (b : ByteArray) (x : Array (Option (Char × Int128))) :
    Bincode.fromBytes b = .ok x ↔ Bincode.toBytes x = .ok b :=
  Bincode.fromBytes_eq_ok_iff

example (b : ByteArray) (m : Std.HashMap String (Array UInt8)) (h : Bincode.toBytes m = .ok b) :
    ∃ m' : Std.HashMap String (Array UInt8), Bincode.fromBytes b = .ok m' ∧ m'.Equiv m :=
  let ⟨m', h', he⟩ := Bincode.fromBytes_of_toBytes_equiv h
  ⟨m', h', Bincode.equiv_hashMap_iff.mp he⟩

def bytes (xs : List UInt8) : ByteArray := ⟨xs.toArray⟩

def isOk [BEq α] (r : Except Error α) (expected : α) : Bool :=
  match r with
  | .ok x => x == expected
  | .error _ => false

def bcsEncodes [Bcs.Serialize α] (x : α) (expected : List UInt8) : Bool :=
  match Bcs.toBytes x with
  | .ok b => b == bytes expected
  | .error _ => false

def bcsRoundTrips [Bcs.Serialize α] [Bcs.Deserialize α] [BEq α] (x : α) : Bool :=
  match Bcs.toBytes x with
  | .ok b => match Bcs.fromBytes (α := α) b with
    | .ok y => x == y
    | .error _ => false
  | .error _ => false

def bcsRejects [Bcs.Deserialize α] (input : List UInt8) : Bool :=
  match Bcs.fromBytes (α := α) (bytes input) with
  | .ok _ => false
  | .error _ => true

def bincodeEncodes [Bincode.Serialize α] (x : α) (expected : List UInt8) : Bool :=
  match Bincode.toBytes x with
  | .ok b => b == bytes expected
  | .error _ => false

def bincodeRoundTrips [Bincode.Serialize α] [Bincode.Deserialize α] [BEq α] (x : α) : Bool :=
  match Bincode.toBytes x with
  | .ok b => match Bincode.fromBytes (α := α) b with
    | .ok y => x == y
    | .error _ => false
  | .error _ => false

def bincodeRejects [Bincode.Deserialize α] (input : List UInt8) : Bool :=
  match Bincode.fromBytes (α := α) (bytes input) with
  | .ok _ => false
  | .error _ => true

def serializeMap (entries : Array (String × UInt8)) : Except Error ByteArray :=
  Bcs.Ser.run (Bcs.Ser.map Bcs.Ser.str Bcs.Ser.u8 entries)

def deserializeMap (input : List UInt8) : Except Error (Array (String × UInt8)) :=
  Bcs.De.run (Bcs.De.map Bcs.De.str Bcs.De.u8) (bytes input)

/-- Nested options of the given depth, each wrapped in a container. -/
def nested : Nat → Bcs.Ser Unit
  | 0 => Bcs.Ser.container (Bcs.Ser.u8 0)
  | n + 1 => Bcs.Ser.container (do Bcs.Ser.u8 1; nested n)

/-- A hash map built in a different order, with a different capacity. -/
def hashMap₁ : Std.HashMap String UInt8 := Std.HashMap.ofList [("b", 2), ("a", 1)]
def hashMap₂ : Std.HashMap String UInt8 := ((Std.HashMap.emptyWithCapacity 64).insert "a" 1).insert "b" 2

def tests : List (String × Bool) := [
  ("bcs u8", bcsEncodes (7 : UInt8) [7]),
  ("bcs u32", bcsEncodes (0x01020304 : UInt32) [4, 3, 2, 1]),
  ("bcs i16", bcsEncodes (-2 : Int16) [0xfe, 0xff]),
  ("bcs u128", bcsEncodes (UInt128.ofNat (2 ^ 64 + 1)) [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]),
  ("bcs i128", bcsEncodes (-1 : Int128) (List.replicate 16 0xff)),
  ("bcs i128 round trip", bcsRoundTrips (Int128.ofInt (-(2 ^ 100)))),
  ("bcs bool", bcsEncodes true [1]),
  ("bcs invalid bool", bcsRejects (α := Bool) [2]),
  ("bcs string", bcsEncodes "ab" [2, 0x61, 0x62]),
  ("bcs invalid utf8", bcsRejects (α := String) [1, 0xff]),
  ("bcs uleb128", bcsEncodes (#[] : Array Unit) [0]),
  ("bcs uleb128 large", bcsEncodes (Array.replicate 300 ()) [0xac, 0x02]),
  ("bcs uleb128 non-canonical", bcsRejects (α := Array Unit) [0x80, 0x00]),
  ("bcs uleb128 overflow", bcsRejects (α := Array Unit) [0xff, 0xff, 0xff, 0xff, 0x10]),
  ("bcs length too large", bcsRejects (α := Array Unit) [0x80, 0x80, 0x80, 0x80, 0x08]),
  ("bcs option", bcsEncodes (some (3 : UInt8)) [1, 3]),
  ("bcs invalid option", bcsRejects (α := Option UInt8) [2, 3]),
  ("bcs pair", bcsEncodes ((1 : UInt8), (2 : UInt16)) [1, 2, 0]),
  ("bcs vector", bcsEncodes (#v[(1 : UInt8), 2, 3]) [1, 2, 3]),
  ("bcs struct", bcsEncodes (Point.mk 1 (some "a")) [1, 0, 1, 1, 0x61]),
  ("bcs struct round trip", bcsRoundTrips (Point.mk 513 none)),
  ("bcs trailing bytes", bcsRejects (α := Point) [1, 0, 0, 0]),
  ("bcs eof", bcsRejects (α := Point) [1, 0]),
  ("bcs float unsupported", (Bcs.toBytes (1.0 : Float)).toBool == false),
  ("bcs map sorted", isOk (serializeMap #[("b", 2), ("a", 1), ("ab", 3)])
    (bytes [3, 1, 0x61, 1, 1, 0x62, 2, 2, 0x61, 0x62, 3])),
  ("bcs map duplicate keys", (serializeMap #[("a", 1), ("a", 2)]).toBool == false),
  ("bcs map ordered", isOk (deserializeMap [2, 1, 0x61, 1, 1, 0x62, 2]) #[("a", 1), ("b", 2)]),
  ("bcs map unordered", (deserializeMap [2, 1, 0x62, 2, 1, 0x61, 1]).toBool == false),
  ("bcs map duplicate", (deserializeMap [2, 1, 0x61, 1, 1, 0x61, 2]).toBool == false),
  ("bcs hash map sorted", bcsEncodes hashMap₁ [2, 1, 0x61, 1, 1, 0x62, 2]),
  ("bcs hash map canonical", bcsEncodes hashMap₂ [2, 1, 0x61, 1, 1, 0x62, 2]),
  ("bcs hash map round trip", bcsRoundTrips hashMap₂),
  ("bcs hash map unordered", bcsRejects (α := Std.HashMap String UInt8) [2, 1, 0x62, 2, 1, 0x61, 1]),
  ("bcs max depth", (Bcs.Ser.run (nested (Bcs.maxContainerDepth - 1))).toBool),
  ("bcs depth exceeded", (Bcs.Ser.run (nested Bcs.maxContainerDepth)).toBool == false),
  ("bincode u32", bincodeEncodes (0x01020304 : UInt32) [4, 3, 2, 1]),
  ("bincode string", bincodeEncodes "ab" [2, 0, 0, 0, 0, 0, 0, 0, 0x61, 0x62]),
  ("bincode f64", bincodeRoundTrips (-0.0021 : Float)),
  ("bincode f32", bincodeEncodes (1.0 : Float32) [0, 0, 0x80, 0x3f]),
  ("bincode char", bincodeEncodes '€' [0xe2, 0x82, 0xac]),
  ("bincode char round trip", bincodeRoundTrips '𐍈'),
  ("bincode char overlong", bincodeRejects (α := Char) [0xc0, 0x80]),
  ("bincode char truncated", bincodeRejects (α := Char) [0xe2, 0x82]),
  ("bincode length too large", bincodeRejects (α := Array Unit) [0, 0, 0, 0x80, 0, 0, 0, 0]),
  ("bincode bool", bincodeEncodes false [0]),
  ("bincode invalid bool", bincodeRejects (α := Bool) [2]),
  ("bincode option", bincodeEncodes (some (5 : UInt64)) [1, 5, 0, 0, 0, 0, 0, 0, 0]),
  ("bincode hash map round trip", bincodeRoundTrips hashMap₂),
  ("bincode hash map unordered", isOk (Bincode.fromBytes (α := Std.HashMap String UInt8)
    (bytes [2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0x62, 2,
      1, 0, 0, 0, 0, 0, 0, 0, 0x61, 1]) |>.map (·.toList.length)) 2),
  ("bincode map keeps order", isOk (Bincode.Ser.run (Bincode.Ser.map Bincode.Ser.u8 Bincode.Ser.u8 #[(2, 0), (1, 0)]))
    (bytes [2, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0])),
  ("bincode deep nesting", (Bincode.Ser.run (nested 10000)).toBool)
]

def main : IO UInt32 := do
  let failures := tests.filter (!·.2)
  for (name, _) in failures do
    IO.eprintln s!"test failed: {name}"
  IO.println s!"{tests.length - failures.length}/{tests.length} tests passed"
  return if failures.isEmpty then 0 else 1
