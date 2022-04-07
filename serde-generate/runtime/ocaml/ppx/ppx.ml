open Ppxlib
open Ast_builder.Default

let depth_table : (string, int) Hashtbl.t = Hashtbl.create 509
let current_name = ref ""

type expr = {
  ser: expression;
  de: expression;
}

type ret = {
  is_recursive: bool;
  depth: int;
  exprs: expr;
}

let ser_name s = "Serde.Serialize." ^ s
let de_name s = "Serde.Deserialize." ^ s
let ret ~loc s =
  { is_recursive = false; depth = 0;
    exprs = { ser = evar ~loc (ser_name s); de = evar ~loc (de_name s) } }

let tuple ~loc l =
  let is_recursive = List.exists (fun r -> r.is_recursive) l in
  let depth = List.fold_left (fun acc r -> max acc r.depth) 0 l in
  let ptuple = ppat_tuple ~loc (List.mapi (fun i _ -> pvar ~loc ("t" ^ string_of_int i)) l) in
  let ser =
    pexp_fun ~loc Nolabel None ptuple @@
    eapply ~loc (evar ~loc (ser_name"concat")) [
      elist ~loc @@ List.mapi (fun i r -> eapply ~loc r.exprs.ser [ evar ~loc ("t" ^ string_of_int i) ]) l
    ] in
  let n = List.length l in
  let ptuple_rev = ppat_tuple ~loc (List.mapi (fun i _ -> pvar ~loc ("t" ^ string_of_int (n-i-1))) l) in
  (* tuples are evaluated from the last to the first *)
  let de =
    pexp_fun ~loc Nolabel None (pvar ~loc "b") @@
    pexp_let ~loc Nonrecursive [
      value_binding ~loc ~pat:ptuple_rev ~expr:(pexp_tuple ~loc @@ List.map (fun r ->
          eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@ List.rev l)
    ] @@ pexp_tuple ~loc (List.mapi (fun i _ -> evar ~loc ("t" ^ string_of_int i)) l) in
  { is_recursive; exprs = { ser; de }; depth }

let rec core ~names c =
  let loc = c.ptyp_loc in
  match c.ptyp_desc with
  | Ptyp_var v ->
    { is_recursive = false; depth = 0;
      exprs = { ser = evar ~loc ("_" ^ v ^ "_ser"); de = evar ~loc ("_" ^ v ^ "_de") } }
  | Ptyp_constr ({txt; _}, args) ->
    let id = Longident.name txt in
    base ~loc ~names ~id args
  | Ptyp_tuple l ->
    let l = List.map (core ~names) l in
    tuple ~loc l
  | _ -> assert false

and base ~loc ~names ~id args = match id, args with
  | "bool", [] | "Bool.t", [] -> ret ~loc "bool"
  | "string", [] | "String.t", [] -> ret ~loc "string"
  | "bytes", [] | "Bytes.t", [] -> ret ~loc "bytes"
  | "float", [] | "Float.t", [] -> ret ~loc "float64"
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
  | "map", [k; v] | "Map.t", [ k; v ] | "Serde.map", [k; v] | "Serde.Map.t", [k; v] ->
    let rk = core ~names k in
    let rv = core ~names v in
    let is_recursive = rk.is_recursive || rv.is_recursive in
    { is_recursive; depth = max rk.depth rv.depth; exprs = {
          ser = eapply ~loc (evar ~loc (ser_name "map")) [ rk.exprs.ser; rv.exprs.ser ];
          de = eapply ~loc (evar ~loc (de_name "map")) [ rk.exprs.ser; rk.exprs.de; rv.exprs.de ];
        } }
  | _ ->
    let l = List.map (core ~names) args in
    let is_recursive = List.mem id names || List.exists (fun r -> r.is_recursive) l in
    let ser = eapply ~loc (evar ~loc (id ^ "_ser")) (List.map (fun r -> r.exprs.ser) l) in
    let de = eapply ~loc (evar ~loc (id ^ "_de")) (List.map (fun r -> r.exprs.de) l) in
    let depth = Option.value ~default:0 @@ Hashtbl.find_opt depth_table id in
    { is_recursive; exprs = { ser; de }; depth }

let record ~names ~loc ?constructor l =
  let lr = List.map (fun pld -> core ~names pld.pld_type) l in
  let is_recursive = List.exists (fun r -> r.is_recursive) lr in
  let depth = 1 + List.fold_left (fun acc r -> max acc r.depth) 0 lr in
  let p = ppat_record ~loc (List.map (fun pld -> Located.lident ~loc pld.pld_name.txt, pvar ~loc pld.pld_name.txt) l) Closed in
  let fields = List.map (fun pld -> evar ~loc pld.pld_name.txt) l in
  let ser =
    eapply ~loc (evar ~loc (ser_name "concat")) [
      elist ~loc @@ List.map2 (fun (pld, f) r ->
          eapply ~loc:pld.pld_loc r.exprs.ser [ f ]) (List.combine l fields) lr
    ] in
  let ser = if Option.is_some constructor then ser else pexp_fun ~loc Nolabel None p ser in
  let prev = ppat_tuple ~loc (List.map (fun pld -> pvar ~loc pld.pld_name.txt) @@ List.rev l) in
  (* tuples/records are evaluated from the last to the first *)
  let re = pexp_record ~loc (List.map (fun pld -> Located.lident ~loc pld.pld_name.txt, evar ~loc pld.pld_name.txt) l) None in
  let de =
    pexp_let ~loc Nonrecursive [
      value_binding ~loc ~pat:prev ~expr:(pexp_tuple ~loc @@ List.map (fun r ->
          eapply ~loc r.exprs.de [ evar ~loc "b" ]) @@ List.rev lr)
    ] @@ Option.fold ~none:re ~some:(fun id -> pexp_construct ~loc (Located.lident ~loc id) (Some re)) constructor in
  let de = if Option.is_some constructor then de else pexp_fun ~loc Nolabel None (pvar ~loc "b") de in
  { is_recursive; exprs = { ser; de }; depth },
  (if Option.is_some constructor then Some p else None)

let constructor ~loc ~names ~id = function
  | Pcstr_tuple [] -> None
  | Pcstr_tuple [ c ] -> Some (core ~names c, None)
  | Pcstr_tuple l -> Some (core ~names (ptyp_tuple ~loc l), None)
  | Pcstr_record l -> Some (record ~names ~loc ~constructor:id l)

let variant ~names ~loc l =
  let lr = List.mapi (fun i pcd ->
      i, constructor ~names ~loc:pcd.pcd_loc ~id:pcd.pcd_name.txt pcd.pcd_args ) l in
  let is_recursive = List.exists (fun (_, r) -> Option.fold ~none:false ~some:(fun (r, _) -> r.is_recursive) r) lr in
  let depth = 1 + List.fold_left (fun acc (_, r) -> match r with
      | None -> acc
      | Some (r, _) -> max acc r.depth) 0 lr in
  let ser = pexp_function ~loc @@ List.map2 (fun pcd (i, r) ->
      let loc = pcd.pcd_loc in
      let c = Located.lident ~loc pcd.pcd_name.txt in
      let p = Option.map (fun (_, p) -> Option.value ~default:(pvar ~loc "x") p) r in
      case ~guard:None ~lhs:(ppat_construct ~loc c p)
        ~rhs:(eapply ~loc (evar ~loc (ser_name "concat")) [
            elist ~loc @@
            eapply ~loc (evar ~loc @@ ser_name "variant_index") [ eint ~loc i] ::
            Option.fold ~none:[] ~some:(fun (r, o) ->
                Option.fold ~none:[ eapply ~loc r.exprs.ser [evar ~loc "x"] ]
                  ~some:(fun _ -> [ r.exprs.ser ]) o
              ) r ])) l lr in
  let l_expr = elist ~loc @@ List.map2 (fun pcd (_, r) ->
      let loc = pcd.pcd_loc in
      let p = Option.fold ~none:(ppat_any ~loc) ~some:(fun _ -> pvar ~loc "b") r in
      let e = match r with
        | None -> pexp_construct ~loc (Located.lident ~loc pcd.pcd_name.txt) None
        | Some (r, None) ->
          pexp_construct ~loc (Located.lident ~loc pcd.pcd_name.txt) @@
          Some (eapply ~loc r.exprs.de [ evar ~loc "b" ])
        | Some (r, Some _) -> r.exprs.de in
      pexp_fun ~loc Nolabel None p e) l lr in
  let de = pexp_fun ~loc Nolabel None (pvar ~loc "b") @@
    pexp_let ~loc Nonrecursive [
      value_binding ~loc ~pat:(pvar ~loc "tag")
        ~expr:(eapply ~loc (evar ~loc @@ de_name "variant_index") [ evar ~loc "b" ]) ] @@
    pexp_let ~loc Nonrecursive [
      value_binding ~loc ~pat:(pvar ~loc "l") ~expr:l_expr ] @@
    pexp_match ~loc (eapply ~loc (evar ~loc "List.nth_opt") [ evar ~loc "l"; evar ~loc "tag" ]) [
      case ~guard:None ~lhs:(ppat_construct ~loc (Located.lident ~loc "None") None)
        ~rhs:(eapply ~loc (evar ~loc "failwith") [estring ~loc "no case matched"]);
      case ~guard:None ~lhs:(ppat_construct ~loc (Located.lident ~loc "Some") (Some (pvar ~loc "de")))
        ~rhs:(eapply ~loc (evar ~loc "de") [ evar ~loc "b" ]);
    ] in
  {is_recursive; exprs = { ser; de }; depth }

let ptype ~names t =
  let loc = t.ptype_loc in
  match t.ptype_kind, t.ptype_manifest with
  | Ptype_abstract, Some c -> core ~names c
  | Ptype_variant l, _ -> variant ~names ~loc l
  | Ptype_record l, _ -> fst @@ record ~names ~loc l
  | _ -> Location.raise_errorf ~loc "type not handled"

let add_depth ~loc r =
  let f e =
    pexp_fun ~loc Nolabel None (pvar ~loc "b") @@
    pexp_let ~loc Nonrecursive [ value_binding ~loc ~pat:(pvar ~loc "f") ~expr:e ] @@
    pexp_match ~loc (evar ~loc "Serde.max_depth") [
      case ~guard:None ~lhs:(ppat_construct ~loc (Located.lident ~loc "Some") (Some (pvar ~loc "i")))
        ~rhs:(pexp_ifthenelse ~loc (eapply ~loc (evar ~loc "<") [ evar ~loc "i"; eint ~loc r.depth ])
                (eapply ~loc (evar ~loc "failwith") [ estring ~loc "max container depth reached" ])
                (Some (eapply ~loc (evar ~loc "f") [ evar ~loc "b" ])));
      case ~guard:None ~lhs:(ppat_any ~loc) ~rhs:(eapply ~loc (evar ~loc "f") [ evar ~loc "b" ])
    ] in
  f r.exprs.ser, f r.exprs.de

let rec add_params ~loc r = function
  | [] -> add_depth ~loc r
  | ({ptyp_desc=Ptyp_var v; ptyp_loc=loc; _}, _) :: t ->
    let (e_ser, e_de) = add_params ~loc r t in
    pexp_fun ~loc Nolabel None (pvar ~loc ("_" ^ v ^ "_ser")) e_ser,
    pexp_fun ~loc Nolabel None (pvar ~loc ("_" ^ v ^ "_de")) e_de
  | ({ptyp_loc=loc; _}, _) :: _ -> Location.raise_errorf ~loc "param not handled"

let str_gen ~loc ~path:_ (_rec_flag, l) debug =
  let names = List.map (fun t -> t.ptype_name.txt) l in
  let lr = List.map (fun t ->
      current_name := t.ptype_name.txt;
      let r = ptype ~names t in
      Hashtbl.add depth_table t.ptype_name.txt r.depth;
      r) l in
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
