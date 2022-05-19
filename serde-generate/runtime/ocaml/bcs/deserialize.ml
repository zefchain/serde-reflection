(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Common.Misc
include Common.Deserialize

let max_u32 = 1 lsl 32 - 1

let char _ = failwith "char deserialization not implemented"
let float32 _ = failwith "float32 deserialization not implemented"
let float64 _ = failwith "float64 deserialization not implemented"

let uleb128_32 b =
  let rec f ~previous_zero acc b i =
    let v = uint8 b in
    let v = Stdint.Uint8.to_int v.r in
    let v_aux = (v land 0x7f) lsl (7 * i) in
    let acc = acc + v_aux in
    if v land 0x80 <> 0 then f ~previous_zero:(v_aux=0) acc b (i+1)
    else if previous_zero && v_aux=0 then failwith "not minimal representation"
    else acc in
  let i = f ~previous_zero:false 0 b 0 in
  if i < 0 || i > max_u32 then failwith "integer above max u32"
  else i


let length b =
  let i = uleb128_32 b in
  if i < 0 || i > max_length then failwith "integer above max length"
  else i

let variant_index b  = uleb128_32 b

let variable f b = variable length f b
let string b = string length b
let bytes b = bytes length b
let map ser_k de_k de_v = map length ser_k de_k de_v
