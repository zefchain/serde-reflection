(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

type 'a r = {r: 'a; depth: int}

let list_depth l = List.fold_left (fun acc x -> max acc x.depth) 0 l
let list_result l = List.map (fun x -> x.r) l
