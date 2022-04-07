open Stdint

val max_depth: int option
val concat : bytes list -> bytes

val bool : bool -> bytes
val uint8 : uint8 -> bytes
val uint16 : uint16 -> bytes
val uint32 : uint32 -> bytes
val uint64 : uint64 -> bytes
val uint128 : uint128 -> bytes
val int8 : int8 -> bytes
val int16 : int16 -> bytes
val int32 : int32 -> bytes
val int64 : int64 -> bytes
val int128 : int128 -> bytes
val option : ('a -> bytes) -> 'a option -> bytes
val unit : unit -> bytes
val fixed : ('a -> bytes) -> 'a list -> bytes

val char : char -> bytes
val length : int -> bytes
val variant_index : int -> bytes
val float32 : float -> bytes
val float64 : float -> bytes
val variable : ('a -> bytes) -> 'a list -> bytes
val string : string -> bytes
val bytes : bytes -> bytes
val map : ('k -> bytes) -> ('v -> bytes) -> ('k, 'v) Common.Map.t -> bytes
