(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Common.Misc
include Common.Serialize

let max_depth : int option = None
let max_length = 1 lsl 31 - 1

let char (c : char) = {Common.Misc.r=Bytes.make 1 c; depth=0}
let length i =
  if i > max_length then failwith "integer above max length"
  else (uint64 @@ Stdint.Uint64.of_int i).r
let variant_index i = uint32 @@ Stdint.Uint32.of_int i
let float32 f =
  let i = Stdlib.Int32.bits_of_float f in
  int32 i
let float64 f =
  let i = Stdlib.Int64.bits_of_float f in
  int64 i

let variable f l = variable length f l
let string s = string length s
let bytes b = bytes length b
let map ser_k ser_v m = map length ser_k ser_v m
