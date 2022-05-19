(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

module Map = Common.Map
type ('k, 'v) map = ('k, 'v) Map.t

include Common.Misc

let max_depth = Runtime.Serialize.max_depth

let check_depth r = match max_depth with
  | Some md when md < r.depth -> failwith (Format.sprintf "depth above %d" md)
  | _ -> r

module Serialize = struct
  include Runtime.Serialize
  let apply f x = (f x).r
end

module Deserialize = struct
  include Runtime.Deserialize
  let check_length b =
    if Bytes.length b.buffer <> b.offset then
      failwith "buffer not empty"
  let apply f buffer =
    let b = {buffer; offset=0} in
    let r = f b in
    let () = check_length b in
    r.r
end
