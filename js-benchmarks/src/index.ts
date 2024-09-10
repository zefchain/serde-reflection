import { Bench } from 'tinybench'
import * as ProtobufRegistry from './proto/bench.ts'
import * as BincodeRegistry from './bincode/registry/registry.ts'
import { BincodeSerializer } from './bincode/bincode/bincodeSerializer.ts'
import { BincodeDeserializer } from './bincode/bincode/bincodeDeserializer.ts'

const skeleton = Object.freeze({
	"string": "Lorem ipsum dolor sit amet.",
	"uint32": 9000,
	"inner": {
		"int32": 20161110,
		"innerInner": {
			"long": 1051,
			"enum": 1,
			"sint32": -42
		},
		"outer": {
			"bool": [true, false, false, true, false, false, true],
			"double": 204.8
		}
	},
	"float": 0.25
})

const bench = new Bench({ time: 5_000 })

bench.add('JSON:encode', () => {
	JSON.stringify(skeleton)
})
bench.add('protobuf-js-ts-proto:encode', () => {
	ProtobufRegistry.Test.encode(skeleton).finish()
})

const outer = new BincodeRegistry.Outer(skeleton.inner.outer.bool, skeleton.inner.outer.double)
const enu_m = new BincodeRegistry.EnumVariantONE()
const inner_inner = new BincodeRegistry.InnerInner(BigInt(skeleton.inner.innerInner.long), enu_m, skeleton.inner.innerInner.sint32)
const inner = new BincodeRegistry.Inner(skeleton.inner.int32, inner_inner, outer)
bench.add('serdegen-bincode:encode', () => {
	const bincode_serializer = new BincodeSerializer()
	new BincodeRegistry.Test(skeleton.string, skeleton.uint32, inner, skeleton.float).serialize(bincode_serializer)
	bincode_serializer.getBytes()
})

const json_encoded = JSON.stringify(skeleton)
bench.add('JSON:decode', () => {
	JSON.parse(json_encoded)
})
const pb_encoded = ProtobufRegistry.Test.encode(skeleton).finish()
bench.add('protobuf-js-ts-proto:decode', () => {
	ProtobufRegistry.Test.decode(pb_encoded)
})

const bc_encoded = function () {
	const outer = new BincodeRegistry.Outer(skeleton.inner.outer.bool, skeleton.inner.outer.double)
	const enu_m = new BincodeRegistry.EnumVariantONE()
	const inner_inner = new BincodeRegistry.InnerInner(BigInt(skeleton.inner.innerInner.long), enu_m, skeleton.inner.innerInner.sint32)
	const inner = new BincodeRegistry.Inner(skeleton.inner.int32, inner_inner, outer)

	const bincode_serializer = new BincodeSerializer()
	new BincodeRegistry.Test(skeleton.string, skeleton.uint32, inner, skeleton.float).serialize(bincode_serializer)
	return bincode_serializer.getBytes()
}()
bench.add('serdegen-bincode:decode', () => {
	const deserializer = new BincodeDeserializer(bc_encoded)
	BincodeRegistry.Test.deserialize(deserializer)
})


await bench.warmup()
await bench.run()

console.table(bench.table())