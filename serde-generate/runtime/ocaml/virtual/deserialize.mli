open Stdint

type b = {buffer : bytes; mutable offset: int}

val bool : b -> bool
val uint8 : b -> uint8
val uint16 : b -> uint16
val uint32 : b -> uint32
val uint64 : b -> uint64
val uint128 : b -> uint128
val int8 : b -> int8
val int16 : b -> int16
val int32 : b -> int32
val int64 : b -> int64
val int128 : b -> int128
val option : (b -> 'a) -> b -> 'a option
val unit : b -> unit
val fixed : (b -> 'a) -> int -> b -> 'a list

val char : b -> char
val length : b -> int
val variant_index : b -> int
val float32 : b -> float
val float64 : b -> float
val variable : (b -> 'a) -> b -> 'a list
val string : b -> string
val bytes : b -> bytes
val map : ('k -> bytes) -> (b -> 'k) -> (b -> 'v) -> b -> ('k, 'v) Common.Map.t
