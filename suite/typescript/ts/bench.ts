import { Bench } from 'tinybench'
import * as ProtobufRegistry from './proto/main.ts'
import * as BincodeRegistry from './bincode/registry.ts'
import * as Data from './data.ts'

const ComplexStruct_pb_obj: ProtobufRegistry.ComplexStruct = {
	inner: { a: 42, b: "Hello" },
	flag: true,
	items: [
		{ variant: { $case: "variant_a", variant_a: { value: 10 } } },
		{ variant: { $case: "variant_b", variant_b: { value: "World" } } },
	],
	unit: {},
	newtype: 99,
	tuple: { first: 123, second: 45.67, third: "Test" },
	// @ts-ignore
	map: { 3: 7n }
}

await async function bench_encode() {
	const b = new Bench({ time: 1_000 })

	b.add('JSON:encode', () => {
		JSON.stringify(Data.ComplexStruct_obj)
	})
	b.add('protobuf-js-ts-proto:encode', () => {
		ProtobufRegistry.ComplexStruct.encode(ComplexStruct_pb_obj)
	})
	b.add('serdegen-bincode:encode', () => {
		BincodeRegistry.ComplexStruct.encode(Data.ComplexStruct_obj)
	})
	await b.warmup()
	await b.run()
	console.table(b.table())
}()



await async function bench_decode() {

	const b = new Bench({ time: 1_000 })

	const json_encoded = JSON.stringify(Data.ComplexStruct_obj)
	b.add('JSON:decode', () => {
		JSON.parse(json_encoded)
	})

	const pb_encoded = ProtobufRegistry.ComplexStruct.encode(ComplexStruct_pb_obj).finish()
	b.add('protobuf-js-ts-proto:decode', () => {
		ProtobufRegistry.ComplexStruct.decode(pb_encoded)
	})

	const bincodec_encoded = BincodeRegistry.ComplexStruct.encode(Data.ComplexStruct_obj)
	b.add('serdegen-bincode:decode', () => {
		BincodeRegistry.ComplexStruct.decode(bincodec_encoded)
	})

	await b.warmup()
	await b.run()
	console.table(b.table())
}()

