(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Stdint
open Misc

let max_length = 1 lsl 31 - 1

let bool bo =
  if bo then {r=Bytes.make 1 '\001'; depth=0}
  else {r=Bytes.make 1 '\000'; depth=0}

let uint8 (i : uint8) =
  let b = Bytes.create 1 in
  Uint8.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let uint16 (i : uint16) =
  let b = Bytes.create 2 in
  Uint16.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let uint32 (i : uint32) =
  let b = Bytes.create 4 in
  Uint32.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let uint64 (i : uint64) =
  let b = Bytes.create 8 in
  Uint64.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let uint128 (i : uint128) =
  let b = Bytes.create 16 in
  Uint128.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let int8 (i : int8) =
  let b = Bytes.create 1 in
  Int8.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let int16 (i : int16) =
  let b = Bytes.create 2 in
  Int16.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let int32 (i : int32) =
  let b = Bytes.create 4 in
  Int32.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let int64 (i : int64) =
  let b = Bytes.create 8 in
  Int64.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let int128 (i : int128) =
  let b = Bytes.create 16 in
  Int128.to_bytes_little_endian i b 0;
  {r=b; depth=0}

let option f = function
  | None -> bool false
  | Some x ->
    let r = f x in
    {r with r=Bytes.concat Bytes.empty [ (bool true).r; r.r ]}

let unit () =
  {r=Bytes.empty; depth=0}

let concat l =
  let depth = list_depth l in
  {depth; r = Bytes.concat Bytes.empty @@ list_result l}

let fixed f a =
  concat @@ Array.to_list @@ Array.map f a

let variable length f l =
  concat @@ {r=length (List.length l); depth=0} :: (List.rev @@ List.rev_map f l)

let string length s =
  { r = Bytes.concat Bytes.empty [ length (String.length s); Bytes.of_string s ]; depth=0 }

let bytes length b =
  { r = Bytes.concat Bytes.empty [ length (Bytes.length b); b ]; depth = 0 }

let map length fk fv m =
  variable length (fun (k, v) -> concat [ fk k; fv v ]) @@ Map.bindings m
