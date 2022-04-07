include Common.Serialize

let max_depth : int option = Some 500
let max_u32 = 1 lsl 32 - 1

let char _ = failwith "char serialization not implememted"
let float32 _ = failwith "float32 serialization not implemented"
let float64 _ = failwith "float64 serialization not implemented"

let uleb128_32 (i : int) =
  if i < 0 || i > max_u32 then failwith "integer not in u32 range"
  else
    let rec f x =
      if x < 0x80 then uint8 (Stdint.Uint8.of_int x)
      else
        Bytes.concat Bytes.empty [
          uint8 @@ Stdint.Uint8.of_int ((x land 0x7f) lor 0x80);
          f (x lsr 7)
        ] in
    f i

let length i =
  if i > max_length then failwith "integer above max length"
  else uleb128_32 i

let variant_index i = uleb128_32 i

let variable f l = variable length f l
let string s = string length s
let bytes b  = bytes length b
let map ser_k ser_v m = map length ser_k ser_v m
