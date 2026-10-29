/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Error

/-!
# Serialization and deserialization monads

`Serde.Ser` writes into a `ByteArray`; `Serde.De` reads from a `ByteArray`. These monads are
shared by all binary formats.

Both monads track a container-depth budget. Every container (struct or enum) consumes one unit
of it (see `Ser.container` and `De.container`), and fails when it is exhausted. Formats that
limit the nesting of containers, such as BCS, start with their limit; the others start with a
budget that is never reached in practice. Recursive types are defined by recursion on the
budget, so that their (de)serializers are total.
-/

namespace Serde

/--
Serialization monad: a function of the remaining container-depth budget and of the output
written so far.
-/
structure Ser (α : Type) : Type where
  toFun : Nat → ByteArray → EStateM.Result Error ByteArray α

/-- Read-only state of the deserializer. -/
structure DeContext where
  input : ByteArray
  depthBudget : Nat

/-- Deserialization monad: a function of the context and of the current input position. -/
structure De (α : Type) : Type where
  toFun : DeContext → Nat → EStateM.Result Error Nat α

instance : CoeFun (Ser α) (fun _ => Nat → ByteArray → EStateM.Result Error ByteArray α) :=
  ⟨Ser.toFun⟩

instance : CoeFun (De α) (fun _ => DeContext → Nat → EStateM.Result Error Nat α) :=
  ⟨De.toFun⟩

namespace Ser

@[always_inline, inline] protected def pure (a : α) : Ser α := ⟨fun _ out => .ok a out⟩

@[always_inline, inline] protected def bind (s : Ser α) (k : α → Ser β) : Ser β :=
  ⟨fun d out =>
    match s d out with
    | .ok a out => k a d out
    | .error e out => .error e out⟩

@[always_inline, inline] protected def throw (e : Error) : Ser α := ⟨fun _ out => .error e out⟩

@[always_inline, inline] protected def tryCatch (s : Ser α) (h : Error → Ser α) : Ser α :=
  ⟨fun d out =>
    match s d out with
    | .error e out => h e d out
    | r => r⟩

@[always_inline] instance : Monad Ser where
  pure := Ser.pure
  bind := Ser.bind

instance : MonadExceptOf Error Ser where
  throw := Ser.throw
  tryCatch := Ser.tryCatch

instance : Inhabited (Ser α) := ⟨Ser.throw default⟩

/-- Applies `f` to the output. -/
@[inline] def modify (f : ByteArray → ByteArray) : Ser Unit := ⟨fun _ out => .ok () (f out)⟩

/-- Runs `f` on an empty output and returns what it wrote, leaving the output unchanged. -/
@[inline] def capture (f : Ser Unit) : Ser ByteArray := ⟨fun d out =>
  match f d ByteArray.empty with
  | .ok _ b => .ok b out
  | .error e b => .error e b⟩

/-- Runs `f` applied to the remaining container-depth budget. -/
@[inline] def withDepth (f : Nat → Ser α) : Ser α := ⟨fun d out => f d d out⟩

/-- Wraps the serialization of a container (struct or enum) to enforce the depth budget. -/
@[inline] def container (f : Ser α) : Ser α := ⟨fun budget out =>
  match budget with
  | 0 => .error .containerDepthExceeded out
  | budget + 1 => f budget out⟩

/-- Runs a serializer with the given container-depth budget. -/
def run (f : Ser Unit) (depthBudget : Nat) : Except Error ByteArray :=
  match f depthBudget ByteArray.empty with
  | .ok _ out => .ok out
  | .error e _ => .error e

end Ser

namespace De

@[always_inline, inline] protected def pure (a : α) : De α := ⟨fun _ pos => .ok a pos⟩

@[always_inline, inline] protected def bind (s : De α) (k : α → De β) : De β :=
  ⟨fun ctx pos =>
    match s ctx pos with
    | .ok a pos => k a ctx pos
    | .error e pos => .error e pos⟩

@[always_inline, inline] protected def throw (e : Error) : De α := ⟨fun _ pos => .error e pos⟩

@[always_inline, inline] protected def tryCatch (s : De α) (h : Error → De α) : De α :=
  ⟨fun ctx pos =>
    match s ctx pos with
    | .error e pos => h e ctx pos
    | r => r⟩

@[always_inline] instance : Monad De where
  pure := De.pure
  bind := De.bind

instance : MonadExceptOf Error De where
  throw := De.throw
  tryCatch := De.tryCatch

instance : Inhabited (De α) := ⟨De.throw default⟩

/-- The input being deserialized. -/
@[inline] def input : De ByteArray := ⟨fun ctx pos => .ok ctx.input pos⟩

/-- The current position in the input. -/
@[inline] def position : De Nat := ⟨fun _ pos => .ok pos pos⟩

/-- Reserves `n` bytes of input and returns the position of the first one. -/
@[inline] def advance (n : Nat) : De Nat := ⟨fun ctx pos =>
  if pos + n ≤ ctx.input.size then .ok pos (pos + n) else .error .unexpectedEof pos⟩

/-- Runs `f` applied to the remaining container-depth budget. -/
@[inline] def withDepth (f : Nat → De α) : De α := ⟨fun ctx pos => f ctx.depthBudget ctx pos⟩

/--
Builds the deserializer `f ()` only when it runs. This is used for the recursive occurrences of
a type in its deserializer, which would otherwise be built for every depth up to the budget.
-/
@[inline] def lazy (f : Unit → De α) : De α := ⟨fun ctx pos => (f ()).toFun ctx pos⟩

/-- Wraps the deserialization of a container (struct or enum) to enforce the depth budget. -/
@[inline] def container (f : De α) : De α := ⟨fun ctx pos =>
  match ctx.depthBudget with
  | 0 => .error .containerDepthExceeded pos
  | budget + 1 => f { ctx with depthBudget := budget } pos⟩

/--
Runs a deserializer on the full input with the given container-depth budget, failing if some
bytes are left.
-/
def run (f : De α) (depthBudget : Nat) (input : ByteArray) : Except Error α :=
  match f { input, depthBudget } 0 with
  | .ok x pos =>
    if pos = input.size then .ok x else .error (.trailingBytes (input.size - pos))
  | .error e _ => .error e

end De

end Serde
