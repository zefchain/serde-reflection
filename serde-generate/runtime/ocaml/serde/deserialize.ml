(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

include Common.Deserialize

let char _ = failwith "char deserialization not implemented"
let length _ = failwith "length deserialization not implemented"
let variant_index _ = failwith "variant_index deserialization not implemented"
let float32 _ = failwith "float32 deserialization not implemented"
let float64 _ = failwith "float64 deserialization not implemented"

let variable f b = variable length f b
let string b = string length b
let bytes b = bytes length b
let map ser_k de_k de_v = map length ser_k de_k de_v
