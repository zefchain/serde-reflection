open Stdint

val max_depth: int option
val concat : bytes Common.Misc.r list -> bytes Common.Misc.r

val bool : bool -> bytes Common.Misc.r
val uint8 : uint8 -> bytes Common.Misc.r
val uint16 : uint16 -> bytes Common.Misc.r
val uint32 : uint32 -> bytes Common.Misc.r
val uint64 : uint64 -> bytes Common.Misc.r
val uint128 : uint128 -> bytes Common.Misc.r
val int8 : int8 -> bytes Common.Misc.r
val int16 : int16 -> bytes Common.Misc.r
val int32 : int32 -> bytes Common.Misc.r
val int64 : int64 -> bytes Common.Misc.r
val int128 : int128 -> bytes Common.Misc.r
val option : ('a -> bytes Common.Misc.r) -> 'a option -> bytes Common.Misc.r
val unit : unit -> bytes Common.Misc.r
val fixed : ('a -> bytes Common.Misc.r) -> 'a array -> bytes Common.Misc.r

val char : char -> bytes Common.Misc.r
val length : int -> bytes
val variant_index : int -> bytes Common.Misc.r
val float32 : float -> bytes Common.Misc.r
val float64 : float -> bytes Common.Misc.r
val variable : ('a -> bytes Common.Misc.r) -> 'a list -> bytes Common.Misc.r
val string : string -> bytes Common.Misc.r
val bytes : bytes -> bytes Common.Misc.r
val map : ('k -> bytes Common.Misc.r) -> ('v -> bytes Common.Misc.r) -> ('k, 'v) Common.Map.t -> bytes Common.Misc.r
