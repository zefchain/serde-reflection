open Stdint

let max_length = 1 lsl 31 - 1

let bool bo =
  if bo then Bytes.make 1 '\001'
  else Bytes.make 1 '\000'

let uint8 (i : uint8) =
  let b = Bytes.create 1 in
  Uint8.to_bytes_little_endian i b 0;
  b

let uint16 (i : uint16) =
  let b = Bytes.create 2 in
  Uint16.to_bytes_little_endian i b 0;
  b

let uint32 (i : uint32) =
  let b = Bytes.create 4 in
  Uint32.to_bytes_little_endian i b 0;
  b

let uint64 (i : uint64) =
  let b = Bytes.create 8 in
  Uint64.to_bytes_little_endian i b 0;
  b

let uint128 (i : uint128) =
  let b = Bytes.create 16 in
  Uint128.to_bytes_little_endian i b 0;
  b

let int8 (i : int8) =
  let b = Bytes.create 1 in
  Int8.to_bytes_little_endian i b 0;
  b

let int16 (i : int16) =
  let b = Bytes.create 2 in
  Int16.to_bytes_little_endian i b 0;
  b

let int32 (i : int32) =
  let b = Bytes.create 4 in
  Int32.to_bytes_little_endian i b 0;
  b

let int64 (i : int64) =
  let b = Bytes.create 8 in
  Int64.to_bytes_little_endian i b 0;
  b

let int128 (i : int128) =
  let b = Bytes.create 8 in
  Int128.to_bytes_little_endian i b 0;
  b

let option f = function
  | None -> bool false
  | Some x -> Bytes.concat Bytes.empty [ bool true; f x ]

let unit () = Bytes.empty

let concat l = Bytes.concat Bytes.empty l

let fixed f a =
  concat @@ List.rev @@ List.rev_map f a

let variable length f l =
  concat @@ (length (List.length l)) :: (List.rev @@ List.rev_map f l)

let string length s =
  concat [ length (String.length s); Bytes.of_string s ]

let bytes length b =
  concat [ length (Bytes.length b); b ]

let map length fk fv m =
  variable length (fun (k, v) -> concat [ fk k; fv v ]) @@ Map.bindings m
