module Serialize = Runtime.Serialize
module Deserialize = Runtime.Deserialize
module Map = Common.Map
type ('k, 'v) map = ('k, 'v) Map.t
let max_depth = Serialize.max_depth
