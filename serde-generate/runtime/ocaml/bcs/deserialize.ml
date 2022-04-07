include Common.Deserialize

let max_u32 = 1 lsl 32 - 1

let char _ = failwith "char deserialization not implemented"
let float32 _ = failwith "float32 deserialization not implemented"
let float64 _ = failwith "float64 deserialization not implemented"

let uleb128_32 b =

  let rec f acc b i =
    let v = uint8 b in
    let v = Stdint.Uint8.to_int v in
    let acc = acc + ((v land 0x7f) lsl (7 * i)) in
    if v land 0x80 <> 0 then f acc b (i+1)
    else acc in
  let i = f 0 b 0 in
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
