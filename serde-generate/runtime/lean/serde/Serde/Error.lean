/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/

namespace Serde

/-- Errors raised by binary serializers and deserializers. -/
inductive Error where
  /-- The input ended before the value was fully read. -/
  | unexpectedEof
  /-- Some bytes were left after reading a complete value. -/
  | trailingBytes (count : Nat)
  /-- A boolean was encoded with a byte other than `0` or `1`. -/
  | invalidBool (byte : UInt8)
  /-- An option was encoded with a tag other than `0` or `1`. -/
  | invalidOptionTag (byte : UInt8)
  /-- A string is not valid UTF-8. -/
  | invalidUtf8
  /-- A character is not a valid Unicode scalar value. -/
  | invalidChar
  /-- A sequence length is too large, or a fixed-size array has the wrong length. -/
  | invalidLength (length : Nat)
  /-- An integer encoded as ULEB128 is malformed, non-canonical, or out of range. -/
  | invalidUleb128
  /-- An enum variant index is not known for the given type. -/
  | unknownVariant (typeName : String) (index : Nat)
  /-- Values are nested too deeply. -/
  | containerDepthExceeded
  /-- Map keys are not in strictly increasing order of their encodings. -/
  | invalidMapOrder
  /-- The encoding does not support this kind of value. -/
  | unsupported (what : String)
  /-- Any other error. -/
  | custom (message : String)
  deriving Repr, BEq, Inhabited

instance : ToString Error where
  toString
    | .unexpectedEof => "unexpected end of input"
    | .trailingBytes n => s!"{n} trailing byte(s) after the end of the value"
    | .invalidBool b => s!"invalid boolean byte {b}"
    | .invalidOptionTag b => s!"invalid option tag {b}"
    | .invalidUtf8 => "invalid UTF-8 string"
    | .invalidChar => "invalid character"
    | .invalidLength n => s!"invalid length {n}"
    | .invalidUleb128 => "invalid ULEB128 integer"
    | .unknownVariant t i => s!"unknown variant index {i} for type {t}"
    | .containerDepthExceeded => "exceeded maximum container depth"
    | .invalidMapOrder => "map keys are not in strictly increasing order"
    | .unsupported what => s!"unsupported value: {what}"
    | .custom m => m

end Serde
