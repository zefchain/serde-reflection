open Stdint

type b = {buffer : bytes; mutable offset: int}

let max_length = 1 lsl 31 - 1

let bool b =
  let c = Bytes.get b.buffer b.offset in
  let r =
    if c = '\001' then true
    else if c = '\000' then false
    else failwith (Format.sprintf "character %C is not a boolean" c) in
  b.offset <- b.offset + 1;
  r

let uint8 b =
  let r = Uint8.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 1;
  r

let uint16 b =
  let r = Uint16.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 2;
  r

let uint32 b =
  let r = Uint32.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 4;
  r

let uint64 b =
  let r = Uint64.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 8;
  r

let uint128 b =
  let r = Uint128.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 16;
  r

let int8 b =
  let r = Int8.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 1;
  r

let int16 b =
  let r = Int16.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 2;
  r

let int32 b =
  let r = Int32.of_bytes_little_endian b.buffer b.offset in
   b.offset <- b.offset + 4;
  r

let int64 b =
  let r = Int64.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 8;
  r

let int128 b =
  let r = Int128.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 16;
  r

let option f b =
  let bo = bool b in
  if not bo then None
  else
    let x = f b in
    Some x

let unit (_ : b) = ()

let fixed f n b =
  let rec aux b acc i =
    if i = 0 then List.rev acc
    else
      let x = f b in
      aux b (x :: acc) (i-1) in
  aux b [] n

let variable length f b =
  let n = length b in
    let rec aux b acc i =
    if i = 0 then List.rev acc
    else
      let x = f b in
      aux b (x :: acc) (i-1) in
  aux b [] n

let is_utf8_string s =
  let decoder = Uutf.decoder ~encoding:`UTF_8 (`String s) in
  let rec aux decoder =
    match Uutf.decode decoder with
    | `Uchar _ -> aux decoder
    | `End -> true
    | `Malformed _ -> false
    | `Await -> false in
  aux decoder

let string length b =
  let n = length b in
  let r = Bytes.sub b.buffer b.offset n in
  b.offset <- b.offset + n;
  let s = Bytes.to_string r in
  if is_utf8_string s then s else failwith "non utf8 string"

let bytes length b =
  let n = length b in
  let r = Bytes.sub b.buffer b.offset n in
  b.offset <- b.offset + n;
  r

let map length ser_k de_k de_v b =
  let compare k1 k2 = Bytes.compare (ser_k k1) (ser_k k2) in
  snd @@ List.fold_left (fun (last_k, acc) (k, v) ->
      match last_k with
      | Some last_k when compare last_k k >= 0 -> failwith "map not ordered"
      | _ -> (Some k, Map.add ~compare k v acc)) (None, Map.empty) @@
  variable length (fun b -> let (v, k) = de_v b, de_k b in (k, v)) b
