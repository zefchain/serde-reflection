// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

#pragma once

#include <algorithm>
#include <cassert>
#include <limits>

#include "binary.hpp"
#include "serde.hpp"

namespace serde {

// Maximum length supported for BCS sequences and maps.
constexpr size_t BCS_MAX_LENGTH = (1ull << 31) - 1;
constexpr size_t BCS_MAX_CONTAINER_DEPTH = 500;

class BcsSerializer : public BinarySerializer<BcsSerializer> {
    using Parent = BinarySerializer<BcsSerializer>;

    void serialize_u32_as_uleb128(uint32_t);

  public:
    BcsSerializer() : Parent(BCS_MAX_CONTAINER_DEPTH) {}

    void serialize_len(size_t value);
    void serialize_variant_index(uint32_t value);

    static constexpr bool enforce_strict_map_ordering = true;
    void sort_last_entries(std::vector<size_t> offsets);
};

class BcsDeserializer : public BinaryDeserializer<BcsDeserializer> {
    using Parent = BinaryDeserializer<BcsDeserializer>;

    uint32_t deserialize_uleb128_as_u32();

  public:
    BcsDeserializer(std::vector<uint8_t> bytes)
        : Parent(std::move(bytes), BCS_MAX_CONTAINER_DEPTH) {}

    size_t deserialize_len();
    uint32_t deserialize_variant_index();

    static constexpr bool enforce_strict_map_ordering = true;
    void check_that_key_slices_are_increasing(std::tuple<size_t, size_t> key1,
                                              std::tuple<size_t, size_t> key2);
};

inline void BcsSerializer::serialize_u32_as_uleb128(uint32_t value) {
    while (value >= 0x80) {
        bytes_.push_back((uint8_t)((value & 0x7F) | 0x80));
        value = value >> 7;
    }
    bytes_.push_back((uint8_t)value);
}

inline void BcsSerializer::serialize_len(size_t value) {
    if (value > BCS_MAX_LENGTH) {
        throw serde::serialization_error("Length is too large");
    }
    serialize_u32_as_uleb128((uint32_t)value);
}

inline void BcsSerializer::serialize_variant_index(uint32_t value) {
    serialize_u32_as_uleb128(value);
}

inline void BcsSerializer::sort_last_entries(std::vector<size_t> offsets) {
    if (offsets.size() <= 1) {
        return;
    }
    offsets.push_back(bytes_.size());

    std::vector<std::vector<uint8_t>> slices;
    for (auto i = 1; i < offsets.size(); i++) {
        auto start = bytes_.cbegin() + offsets[i - 1];
        auto end = bytes_.cbegin() + offsets[i];
        slices.emplace_back(start, end);
    }

    std::sort(slices.begin(), slices.end(), [](auto &s1, auto &s2) {
        return std::lexicographical_compare(s1.begin(), s1.end(), s2.begin(),
                                            s2.end());
    });

    bytes_.resize(offsets[0]);
    for (auto slice : slices) {
        bytes_.insert(bytes_.end(), slice.begin(), slice.end());
    }
    assert(offsets.back() == bytes_.size());
}

inline uint32_t BcsDeserializer::deserialize_uleb128_as_u32() {
    uint64_t value = 0;
    for (int shift = 0; shift < 32; shift += 7) {
        auto byte = read_byte();
        auto digit = byte & 0x7F;
        value |= (uint64_t)digit << shift;
        if (value > std::numeric_limits<uint32_t>::max()) {
            throw serde::deserialization_error(
                "Overflow while parsing uleb128-encoded uint32 value");
        }
        if (digit == byte) {
            if (shift > 0 && digit == 0) {
                throw serde::deserialization_error(
                    "Invalid uleb128 number (unexpected zero digit)");
            }
            return (uint32_t)value;
        }
    }
    throw serde::deserialization_error(
        "Overflow while parsing uleb128-encoded uint32 value");
}

inline size_t BcsDeserializer::deserialize_len() {
    auto value = deserialize_uleb128_as_u32();
    if (value > BCS_MAX_LENGTH) {
        throw serde::deserialization_error("Length is too large");
    }
    return (size_t)value;
}

inline uint32_t BcsDeserializer::deserialize_variant_index() {
    return deserialize_uleb128_as_u32();
}

inline void BcsDeserializer::check_that_key_slices_are_increasing(
    std::tuple<size_t, size_t> key1, std::tuple<size_t, size_t> key2) {
    if (!std::lexicographical_compare(bytes_.cbegin() + std::get<0>(key1),
                                      bytes_.cbegin() + std::get<1>(key1),
                                      bytes_.cbegin() + std::get<0>(key2),
                                      bytes_.cbegin() + std::get<1>(key2))) {
        throw serde::deserialization_error(
            "Error while decoding map: keys are not serialized in the "
            "expected order");
    }
}

} // end of namespace serde
