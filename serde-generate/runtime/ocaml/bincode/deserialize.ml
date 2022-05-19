(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Common.Misc
include Common.Deserialize

let max_length = 1 lsl 31 - 1

let char b =
  let c = Bytes.get b.buffer b.offset in
  b.offset <- b.offset + 1;
  {Common.Misc.r=c; depth=0}

let length b =
  let i = Stdint.Uint64.to_int @@ (uint64 b).r in
  if i < 0 || i > max_length then failwith "integer above max length"
  else i

let variant_index b = Stdint.Uint32.to_int @@ (uint32 b).r

let float32 b =
  let i = int32 b in
  { i with r = Stdlib.Int32.float_of_bits i.r }

let float64 b =
  let i = int64 b in
  { i with r = Stdlib.Int64.float_of_bits i.r }

let variable f b = variable length f b
let string b = string length b
let bytes b = bytes length b
let map ser_k de_k de_v b = map length ser_k de_k de_v b
