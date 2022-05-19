(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Stdint
open Misc

type b = {buffer : bytes; mutable offset: int}

let max_length = 1 lsl 31 - 1

let bool b =
  let c = Bytes.get b.buffer b.offset in
  let r =
    if c = '\001' then true
    else if c = '\000' then false
    else failwith (Format.sprintf "character %C is not a boolean" c) in
  b.offset <- b.offset + 1;
  {r; depth=0}

let uint8 b =
  let r = Uint8.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 1;
  {r; depth=0}

let uint16 b =
  let r = Uint16.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 2;
  {r; depth=0}

let uint32 b =
  let r = Uint32.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 4;
  {r; depth=0}

let uint64 b =
  let r = Uint64.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 8;
  {r; depth=0}

let uint128 b =
  let r = Uint128.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 16;
  {r; depth=0}

let int8 b =
  let r = Int8.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 1;
  {r; depth=0}

let int16 b =
  let r = Int16.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 2;
  {r; depth=0}

let int32 b =
  let r = Int32.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 4;
  {r; depth=0}

let int64 b =
  let r = Int64.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 8;
  {r; depth=0}

let int128 b =
  let r = Int128.of_bytes_little_endian b.buffer b.offset in
  b.offset <- b.offset + 16;
  {r; depth=0}

let option f b =
  let bo = bool b in
  if not bo.r then {r=None; depth=0}
  else
    let r = f b in
    {r with r=Some r.r}

let unit (_ : b) = {r=(); depth=0}

let fixed f n b =
  let rec aux b acc i =
    if i = 0 then {acc with r = Array.of_list @@ List.rev acc.r}
    else
      let x = f b in
      let depth = max acc.depth x.depth in
      aux b {r=x.r :: acc.r; depth} (i-1) in
  aux b {r=[]; depth=0} n

let variable length f b =
  let n = length b in
  let rec aux b acc i =
    if i = 0 then {acc with r = List.rev acc.r}
    else
      let x = f b in
      let depth = max acc.depth x.depth in
      aux b {r=x.r :: acc.r; depth} (i-1) in
  aux b {r=[]; depth=0} n

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
  if is_utf8_string s then {r=s; depth=0} else failwith "non utf8 string"

let bytes length b =
  let n = length b in
  let r = Bytes.sub b.buffer b.offset n in
  b.offset <- b.offset + n;
  {r; depth=0}

let map length ser_k de_k de_v b =
  let compare k1 k2 = Bytes.compare (ser_k k1).r (ser_k k2).r in
  let r = variable length (fun b ->
      let (v, k) = de_v b, de_k b in
      {depth = max k.depth v.depth; r = (k.r, v.r)}) b in
  { r with
    r = snd @@ List.fold_left (fun (last_k, acc) (k, v) ->
        match last_k with
        | Some last_k when compare last_k k >= 0 -> failwith "map not ordered"
        | _ -> (Some k, Map.add ~compare k v acc)) (None, Map.empty) r.r }
