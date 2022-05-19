(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

include Common.Serialize

let max_depth : int option = None

let char _ = failwith "char serialization not implememted"
let length _ = failwith "length serialization not implemented"
let variant_index _ = failwith "variant_index serialization not implemented"
let float32 _ = failwith "float32 serialization not implemented"
let float64 _ = failwith "float64 serialization not implemented"

let variable f l = variable length f l
let string s = string length s
let bytes b = bytes length b
let map ser_k ser_v m = map length ser_k ser_v m
