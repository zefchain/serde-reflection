(* Copyright (c) Zefchain Labs, Inc.
 * SPDX-License-Identifier: MIT OR Apache-2.0 *)

open Ppxlib
open Ast_builder.Default

type expr = {
  ser: expression;
  de: expression;
}

type ret = {
  is_recursive: bool;
  exprs: expr;
}

let lid ~loc s = Located.lident ~loc s
let efun ~loc p e = pexp_fun ~loc Nolabel None p e
let elet ~loc p expr = pexp_let ~loc Nonrecursive [ value_binding ~loc ~pat:(pvar ~loc p) ~expr ]

let ser_name s = "Serde.Serialize." ^ s
let de_name s = "Serde.Deserialize." ^ s
let ret ~loc s =
  { is_recursive = false;
    exprs = { ser = evar ~loc (ser_name s); de = evar ~loc (de_name s) } }

let check_depth ~loc r = eapply ~loc (evar ~loc "Serde.check_depth") [ r ]

let tuple ~loc l =
  let is_recursive = List.exists (fun r -> r.is_recursive) l in
  let ptuple = ppat_tuple ~loc (List.mapi (fun i _ -> pvar ~loc ("t" ^ string_of_int i)) l) in
  let ser =
    efun ~loc ptuple @@
    eapply ~loc (evar ~loc (ser_name"concat")) [
      elist ~loc @@ List.mapi (fun i r -> eapply ~loc r.exprs.ser [ evar ~loc ("t" ^ string_of_int i) ]) l
    ] in
  let r_de =
    pexp_let ~loc Nonrecursive (List.mapi (fun i r ->
        value_binding ~loc ~pat:(pvar ~loc ("t" ^ string_of_int i)) ~expr:(
          elet ~loc "x" (eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@
          pexp_sequence ~loc (eapply ~loc (evar ~loc ":=") [
              evar ~loc "depth";
              eapply ~loc (evar ~loc "max") [
                eapply ~loc (evar ~loc "!") [ evar ~loc "depth"];
                pexp_field ~loc (evar ~loc "x") (lid ~loc "Serde.depth") ] ])
            (pexp_field ~loc (evar ~loc "x") (lid ~loc "Serde.r")))) l) @@
    pexp_tuple ~loc (List.mapi (fun i _ -> evar ~loc ("t" ^ string_of_int i)) l) in
  let de =
    efun ~loc (pvar ~loc "b") @@
    elet ~loc "depth" (eapply ~loc (evar ~loc "ref") [ eint ~loc 0 ]) @@
    elet ~loc "r" r_de @@
    pexp_record ~loc [
      lid ~loc "Serde.depth", eapply ~loc (evar ~loc "!") [ evar ~loc "depth" ];
      lid ~loc "Serde.r", evar ~loc "r" ] None in
  { is_recursive; exprs = { ser; de } }

let is_struct l =
  List.exists (fun a -> a.attr_name.txt = "struct") l

let incr_depth ~loc e =
  elet ~loc "r" e @@
  check_depth ~loc @@
  pexp_record ~loc [
    lid ~loc "Serde.depth", eapply ~loc (evar ~loc "+") [
      pexp_field ~loc (evar ~loc "r") (lid ~loc "Serde.depth");
      eint ~loc 1
    ]
  ] (Some (evar ~loc "r"))

let rec core ~names c =
  let loc = c.ptyp_loc in
  let r = match c.ptyp_desc with
    | Ptyp_var v ->
      { is_recursive = false;
        exprs = { ser = evar ~loc ("_" ^ v ^ "_ser"); de = evar ~loc ("_" ^ v ^ "_de") } }
    | Ptyp_constr ({txt; _}, args) ->
      let id = Longident.name txt in
      base ~attrs:c.ptyp_attributes ~loc ~names ~id args
    | Ptyp_tuple l ->
      let l = List.map (core ~names) l in
      tuple ~loc l
    | _ -> assert false in
  if is_struct c.ptyp_attributes then
    let ser = efun ~loc (pvar ~loc "x") @@
      incr_depth ~loc (eapply ~loc r.exprs.ser [ evar ~loc "x" ]) in
    let de = efun ~loc (pvar ~loc "b") @@
      incr_depth ~loc (eapply ~loc r.exprs.de [ evar ~loc "b" ]) in
    { r with exprs = {ser; de} }
  else r

and base ?(attrs=[]) ~loc ~names ~id args = match id, args with
  | "bool", [] | "Bool.t", [] -> ret ~loc "bool"
  | "string", [] | "String.t", [] -> ret ~loc "string"
  | "bytes", [] | "Bytes.t", [] -> ret ~loc "bytes"
  | "float", [] | "Float.t", [] ->
    if List.exists (fun a -> a.attr_name.txt = "float32") attrs then ret ~loc "float32"
    else ret ~loc "float64"
  | "char", [] | "Char.t", [] -> ret ~loc "char"
  | "unit", [] -> ret ~loc "unit"
  | "uint8", [] | "Stdint.uint8", [] | "Stdint.Uint8.t", [] -> ret ~loc "uint8"
  | "uint16", [] | "Stdint.uint16", [] | "Stdint.Uint16.t", [] -> ret ~loc "uint16"
  | "uint32", [] | "Stdint.uint32", [] | "Stdint.Uint32.t", [] -> ret ~loc "uint32"
  | "uint64", [] | "Stdint.uint64", [] | "Stdint.Uint64.t", [] -> ret ~loc "uint64"
  | "uint128", [] | "Stdint.uint128", [] | "Stdint.Uint128.t", [] -> ret ~loc "uint128"
  | "int8", [] | "Stdint.int8", [] | "Stdint.Int8.t", [] -> ret ~loc "int8"
  | "int16", [] | "Stdint.int16", [] | "Stdint.Int16.t", [] -> ret ~loc "int16"
  | "int32", [] | "Stdint.int32", [] | "Stdint.Int32.t", [] -> ret ~loc "int32"
  | "int64", [] | "Stdint.int64", [] | "Stdint.Int64.t", [] -> ret ~loc "int64"
  | "int128", [] | "Stdint.int128", [] | "Stdint.Int128.t", [] -> ret ~loc "int128"
  | "option", [ c ] | "Option.t", [ c ] ->
    let r = core ~names c in
    { r with exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "option")) [ r.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "option")) [ r.exprs.de ] } }
  | "list", [ c ] | "List.t", [ c ] ->
    let r = core ~names c in
    { r with exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "variable")) [ r.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "variable")) [ r.exprs.de ] } }
  | "array", [ c ] | "Array.t", [ c ] ->
    let r = core ~names c in
    let length = List.find_map (fun a -> match a.attr_name.txt, a.attr_payload with
        | "length", PStr [{pstr_desc=Pstr_eval ({pexp_desc=Pexp_constant (Pconst_integer (s, _)); _}, _); _}] ->
          Some (int_of_string s)
        | _ -> None) attrs in
    begin match length with
      | None ->
        { r with exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "variable")) [ r.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "variable")) [ r.exprs.de ] } }
      | Some length ->
        { r with exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "fixed")) [ r.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "fixed")) [ r.exprs.de; eint ~loc length ] } }
    end
  | "map", [k; v] | "Map.t", [ k; v ] | "Serde.map", [k; v] | "Serde.Map.t", [k; v] ->
    let rk = core ~names k in
    let rv = core ~names v in
    let is_recursive = rk.is_recursive || rv.is_recursive in
    { is_recursive; exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "map")) [ rk.exprs.ser; rv.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "map")) [ rk.exprs.ser; rk.exprs.de; rv.exprs.de ];
        } }
  | _ ->
    let l = List.map (core ~names) args in
    let is_recursive = List.mem id names || List.exists (fun r -> r.is_recursive) l in
    let ser = eapply ~loc (evar ~loc (id ^ "_ser")) (List.map (fun r -> r.exprs.ser) l) in
    let de = eapply ~loc (evar ~loc (id ^ "_de")) (List.map (fun r -> r.exprs.de) l) in
    { is_recursive; exprs = { ser; de } }

let record ~names ~loc ?constructor l =
  let lr = List.map (fun pld -> core ~names pld.pld_type) l in
  let is_recursive = List.exists (fun r -> r.is_recursive) lr in
  let p = ppat_record ~loc (List.map (fun pld -> Located.lident ~loc pld.pld_name.txt, pvar ~loc pld.pld_name.txt) l) Closed in
  let fields = List.map (fun pld -> evar ~loc pld.pld_name.txt) l in
  let r_ser = eapply ~loc (evar ~loc (ser_name "concat")) [
      elist ~loc @@ List.map2 (fun (pld, f) r ->
      eapply ~loc:pld.pld_loc r.exprs.ser [ f ]) (List.combine l fields) lr ] in
  let ser = match constructor with
    | None -> efun ~loc p @@ incr_depth ~loc r_ser
    | Some _ -> r_ser in
  let r_de =
    let re =
      pexp_record ~loc (List.map (fun pld ->
          lid ~loc pld.pld_name.txt, evar ~loc pld.pld_name.txt) l) None in
    pexp_let ~loc Nonrecursive (List.map2 (fun pld r ->
        value_binding ~loc ~pat:(pvar ~loc pld.pld_name.txt) ~expr:(
          elet ~loc "r" (eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@
          pexp_sequence ~loc (eapply ~loc (evar ~loc ":=") [
              evar ~loc "depth";
              eapply ~loc (evar ~loc "max") [
                eapply ~loc (evar ~loc "!") [ evar ~loc "depth"];
                pexp_field ~loc (evar ~loc "r") (lid ~loc "Serde.depth") ] ])
            (pexp_field ~loc (evar ~loc "r") (lid ~loc "Serde.r")))) l lr) @@
    Option.fold ~none:re ~some:(fun id -> pexp_construct ~loc (Located.lident ~loc id) (Some re)) constructor in
  let de e =
    elet ~loc "depth" (eapply ~loc (evar ~loc "ref") [ eint ~loc 0 ]) @@
    elet ~loc "r" r_de e in
  let de_end =
    pexp_record ~loc [
      Located.lident ~loc "Serde.depth",
      eapply ~loc (evar ~loc "+") [
        eapply ~loc (evar ~loc "!") [ evar ~loc "depth" ]; eint ~loc 1 ];
      Located.lident ~loc "Serde.r", evar ~loc "r" ] None in
  let de = match constructor with
    | None ->
      efun ~loc (pvar ~loc "b") @@
      de @@ check_depth ~loc @@ de_end
    | Some _ ->
      de de_end in
  { is_recursive; exprs = { ser; de } },
  (if Option.is_some constructor then Some p else None)

let constructor ~loc ~names ~id = function
  | Pcstr_tuple [] -> None
  | Pcstr_tuple [ c ] -> Some (core ~names c, None)
  | Pcstr_tuple l -> Some (core ~names (ptyp_tuple ~loc l), None)
  | Pcstr_record l -> Some (record ~names ~loc ~constructor:id l)

let is_cyclic l =
  List.find_map (fun pcd ->
      if List.exists (fun a -> a.attr_name.txt = "cyclic") pcd.pcd_attributes then
        match pcd.pcd_args with
        | Pcstr_tuple [ x ] -> Some (pcd.pcd_name.txt, x)
        | _ -> None
      else None) l

let variant ~names ~loc l =
  match is_cyclic l with
  | Some (name, c) ->
    let r = core ~names c in
    let nlid = lid ~loc name in
    let ser = efun ~loc (pvar ~loc "x") @@
      pexp_let ~loc Nonrecursive [
        value_binding ~loc ~pat:(ppat_construct ~loc nlid (Some (pvar ~loc "x"))) ~expr:(evar ~loc "x") ] @@
      incr_depth ~loc (eapply ~loc r.exprs.ser [ evar ~loc "x" ]) in
    let de = efun ~loc (pvar ~loc "b") @@
      elet ~loc "r" (eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@
      check_depth ~loc @@
      pexp_record ~loc [
        lid ~loc "Serde.r",
        pexp_construct ~loc nlid (Some (pexp_field ~loc (evar ~loc "r") (lid ~loc "Serde.r")));
        lid ~loc "Serde.depth",
        eapply ~loc (evar ~loc "+") [
          pexp_field ~loc (evar ~loc "r") (lid ~loc "Serde.depth");
          eint ~loc 1 ]
      ] None in
    {r with exprs = {ser; de}}
  | None ->
    let lr = List.mapi (fun i pcd ->
        i, constructor ~names ~loc:pcd.pcd_loc ~id:pcd.pcd_name.txt pcd.pcd_args ) l in
    let is_recursive = List.exists (fun (_, r) -> Option.fold ~none:false ~some:(fun (r, _) -> r.is_recursive) r) lr in
    let ser =
      efun ~loc (pvar ~loc "x") @@
      incr_depth ~loc (
        pexp_match ~loc (evar ~loc "x") @@ List.map2 (fun pcd (i, r) ->
            let loc = pcd.pcd_loc in
            let c = lid ~loc pcd.pcd_name.txt in
            let p = Option.map (fun (_, p) -> Option.value ~default:(pvar ~loc "x") p) r in
            case ~guard:None ~lhs:(ppat_construct ~loc c p)
              ~rhs:(eapply ~loc (evar ~loc (ser_name "concat")) [
                  elist ~loc @@
                  eapply ~loc (evar ~loc @@ ser_name "variant_index") [ eint ~loc i] ::
                  Option.fold ~none:[] ~some:(fun (r, o) ->
                      Option.fold ~none:[ eapply ~loc r.exprs.ser [evar ~loc "x"] ]
                        ~some:(fun _ -> [ r.exprs.ser ]) o
                    ) r ])) l lr) in
    let l_expr = elist ~loc @@ List.map2 (fun pcd (_, r) ->
        let loc = pcd.pcd_loc in
        let p = Option.fold ~none:(ppat_any ~loc) ~some:(fun _ -> pvar ~loc "b") r in
        let e = match r with
          | None ->
            pexp_record ~loc [
              lid ~loc "Serde.r",
              pexp_construct ~loc (lid ~loc pcd.pcd_name.txt) None;
              lid ~loc "Serde.depth", eint ~loc 1 ] None
          | Some (r, None) ->
            elet ~loc "r" (eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@
            pexp_record ~loc [
              Located.lident ~loc "Serde.r", pexp_construct ~loc (Located.lident ~loc pcd.pcd_name.txt)
                (Some (pexp_field ~loc (evar ~loc "r") (Located.lident ~loc "Serde.r")));
              Located.lident ~loc "Serde.depth",
              eapply ~loc (evar ~loc "+") [
                pexp_field ~loc (evar ~loc "r") (Located.lident ~loc "Serde.depth");
                eint ~loc 1 ]
            ] None
          | Some (r, Some _) -> r.exprs.de in
        efun ~loc p e) l lr in
    let de = efun ~loc (pvar ~loc "b") @@
      elet ~loc "tag" (eapply ~loc (evar ~loc @@ de_name "variant_index") [ evar ~loc "b" ]) @@
      elet ~loc "l" l_expr @@
      pexp_match ~loc (eapply ~loc (evar ~loc "List.nth_opt") [ evar ~loc "l"; evar ~loc "tag" ]) [
        case ~guard:None ~lhs:(ppat_construct ~loc (lid ~loc "None") None)
          ~rhs:(eapply ~loc (evar ~loc "failwith") [estring ~loc "no case matched"]);
        case ~guard:None ~lhs:(ppat_construct ~loc (lid ~loc "Some") (Some (pvar ~loc "de")))
          ~rhs:(check_depth ~loc (eapply ~loc (evar ~loc "de") [ evar ~loc "b" ]));
      ] in
    {is_recursive; exprs = { ser; de } }

let ptype ~names t =
  let loc = t.ptype_loc in
  match t.ptype_kind, t.ptype_manifest with
  | Ptype_abstract, Some c -> core ~names c
  | Ptype_variant l, _ -> variant ~names ~loc l
  | Ptype_record l, _ -> fst @@ record ~names ~loc l
  | _ -> Location.raise_errorf ~loc "type not handled"

let rec add_params ~loc:_ r = function
  | [] -> r.exprs.ser, r.exprs.de
  | ({ptyp_desc=Ptyp_var v; ptyp_loc=loc; _}, _) :: t ->
    let (e_ser, e_de) = add_params ~loc r t in
    efun ~loc (pvar ~loc ("_" ^ v ^ "_ser")) e_ser,
    efun ~loc (pvar ~loc ("_" ^ v ^ "_de")) e_de
  | ({ptyp_loc=loc; _}, _) :: _ -> Location.raise_errorf ~loc "param not handled"

let str_gen ~loc ~path:_ (_rec_flag, l) debug =
  let names = List.map (fun t -> t.ptype_name.txt) l in
  let lr = List.map (ptype ~names) l in
  let rec_flag =
    if List.exists (fun r -> r.is_recursive) lr then Recursive
    else Nonrecursive in
  let l = List.map2 (fun t r ->
      let loc = t.ptype_loc in
      let name = t.ptype_name.txt in
      let ser, de = add_params ~loc r t.ptype_params in [
        value_binding ~loc ~pat:(pvar ~loc (name ^ "_ser")) ~expr:ser;
        value_binding ~loc ~pat:(pvar ~loc (name ^ "_de")) ~expr:de;
      ]) l lr in
  let str = [ pstr_value ~loc rec_flag @@ List.flatten l ] in
  if debug then Format.printf "%s@." @@ Pprintast.string_of_structure str;
  str

let () =
  let args_str = Deriving.Args.(empty +> flag "debug") in
  let str_type_decl = Deriving.Generator.make args_str str_gen in
  Deriving.ignore @@ Deriving.add "serde" ~str_type_decl
