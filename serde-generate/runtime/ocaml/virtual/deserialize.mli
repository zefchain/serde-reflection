open Stdint

type b = {buffer : bytes; mutable offset: int}

val bool : b -> bool Common.Misc.r
val uint8 : b -> uint8 Common.Misc.r
val uint16 : b -> uint16 Common.Misc.r
val uint32 : b -> uint32 Common.Misc.r
val uint64 : b -> uint64 Common.Misc.r
val uint128 : b -> uint128 Common.Misc.r
val int8 : b -> int8 Common.Misc.r
val int16 : b -> int16 Common.Misc.r
val int32 : b -> int32 Common.Misc.r
val int64 : b -> int64 Common.Misc.r
val int128 : b -> int128 Common.Misc.r
val option : (b -> 'a Common.Misc.r) -> b -> 'a option Common.Misc.r
val unit : b -> unit Common.Misc.r
val fixed : (b -> 'a Common.Misc.r) -> int -> b -> 'a array Common.Misc.r

val char : b -> char Common.Misc.r
val length : b -> int
val variant_index : b -> int
val float32 : b -> float Common.Misc.r
val float64 : b -> float Common.Misc.r
val variable : (b -> 'a Common.Misc.r) -> b -> 'a list Common.Misc.r
val string : b -> string Common.Misc.r
val bytes : b -> bytes Common.Misc.r
val map : ('k -> bytes Common.Misc.r) -> (b -> 'k Common.Misc.r) -> (b -> 'v Common.Misc.r) -> b -> ('k, 'v) Common.Map.t Common.Misc.r
