include Common.Deserialize

let max_length = 1 lsl 31 - 1

let char b =
  let c = Bytes.get b.buffer b.offset in
  b.offset <- b.offset + 1;
  c

let length b =
  let i = Stdint.Uint64.to_int @@ uint64 b in
  if i < 0 || i > max_length then failwith "integer above max length"
  else i

let variant_index b = Stdint.Uint32.to_int @@ uint32 b

let float32 b =
  let i = int32 b in
  Stdlib.Int32.float_of_bits i

let float64 b =
  let i = int64 b in
  Stdlib.Int64.float_of_bits i

let variable f b = variable length f b
let string b = string length b
let bytes b = bytes length b
let map ser_k de_k de_v b = map length ser_k de_k de_v b
