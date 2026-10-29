/-
Copyright (c) Zefchain Labs, Inc.
SPDX-License-Identifier: MIT OR Apache-2.0
-/
import Serde.Lawful.Combinators

/-!
# Correctness of primitive types

Fixed-width integers, booleans, ULEB128-encoded integers and lengths, byte arrays,
strings, and the unsupported types (characters and floats).
-/

namespace Serde

/-! ## Basic steps -/

theorem ByteArray.get!_eq_getElem! (b : ByteArray) (i : Nat) : b.get! i = b[i]! := by
  cases b with
  | mk data =>
    simp only [ByteArray.get!]
    by_cases h : i < data.size
    · rw [getElem!_pos (ByteArray.mk data) i h, getElem!_pos data i h]
      rfl
    · rw [getElem!_neg (ByteArray.mk data) i h, getElem!_neg data i h]

theorem Ser.writes_modify {f : ByteArray → ByteArray} (hf : ∀ out, f out = out ++ e) :
    (Ser.modify f).Writes d e := fun out => by
  show EStateM.Result.ok () (f out) = _
  rw [hf]

theorem Ser.fails_throw : (throw err : Ser Unit).Fails d := fun _ => ⟨_, _, rfl⟩

theorem Ser.writes_u8 : (Ser.u8 x).Writes d [x].toByteArray :=
  Ser.writes_modify fun _ => ByteArray.append_toByteArray_singleton.symm

theorem Ser.writes_raw : (Ser.raw b).Writes d b := Ser.writes_modify fun _ => rfl

namespace De

theorem advance_eq_ok : advance n ctx pos = .ok p pos' ↔
    pos + n ≤ ctx.input.size ∧ p = pos ∧ pos' = pos + n := by
  unfold advance
  dsimp only
  split
  · simp_all [eq_comm]
  · simp only [reduceCtorEq, false_iff]
    omega

theorem input_eq_ok : input ctx pos = .ok b pos' ↔ b = ctx.input ∧ pos' = pos := by
  simp [input, eq_comm]

theorem position_eq_ok : position ctx pos = .ok p pos' ↔ p = pos ∧ pos' = pos := by
  simp [position, eq_comm]

theorem throw_eq_ok : (throw e : De α) ctx pos = .ok y pos' ↔ False := by
  simp [throw, throwThe, MonadExceptOf.throw, De.throw]

/-- Characterizes deserializers that read `n` bytes and decode them with `get`. -/
theorem fixed_eq_ok {get : ByteArray → Nat → α} :
    (do let p ← advance n; return get (← input) p : De α) ctx pos = .ok y pos' ↔
      pos + n ≤ ctx.input.size ∧ y = get ctx.input pos ∧ pos' = pos + n := by
  simp only [bind_eq_ok, advance_eq_ok, input_eq_ok, pure_eq_ok]
  constructor
  · rintro ⟨p, p', ⟨hle, rfl, rfl⟩, b, p'', ⟨rfl, rfl⟩, rfl, rfl⟩
    exact ⟨hle, rfl, rfl⟩
  · rintro ⟨hle, rfl, rfl⟩
    exact ⟨_, _, ⟨hle, rfl, rfl⟩, _, _, ⟨rfl, rfl⟩, rfl, rfl⟩

theorem u8_eq_ok : u8 ctx pos = .ok y pos' ↔
    pos + 1 ≤ ctx.input.size ∧ y = ctx.input[pos]! ∧ pos' = pos + 1 := by
  rw [← ByteArray.get!_eq_getElem!]
  exact fixed_eq_ok

end De

/-! ## Byte slices as lists -/

theorem extract_eq_ofFn {b : ByteArray} {p n : Nat} (h : p + n ≤ b.size) :
    b.extract p (p + n) = (List.ofFn fun i : Fin n => b[p + i]!).toByteArray := by
  apply ByteArray.ext_getElem
  · simp only [ByteArray.size_extract, List.size_toByteArray, List.length_ofFn]
    omega
  · intro i hi hi'
    simp only [ByteArray.getElem_extract, List.getElem_toByteArray, List.getElem_ofFn]
    rw [getElem!_pos]

theorem getElem!_of_extract {b : ByteArray} {l : List UInt8} {p : Nat}
    (h : b.extract p (p + l.length) = l.toByteArray) (i : Nat) (hi : i < l.length) :
    b[p + i]! = l[i] := by
  have hsize := congrArg ByteArray.size h
  simp only [ByteArray.size_extract, List.size_toByteArray] at hsize
  have := congrArg (fun a : ByteArray => a[i]!) h
  rw [getElem!_pos _ _ (by simp [ByteArray.size_extract]; omega), ByteArray.getElem_extract,
    getElem!_pos _ _ (by simpa using hi), List.getElem_toByteArray] at this
  rw [getElem!_pos _ _ (by omega)]
  exact this

/-! ## Fixed-width values -/

/--
Correctness of a serializer writing `n` bytes `enc x`, and of a deserializer reading `n`
bytes and decoding them with `get`.
-/
theorem correct_fixed {ser : α → Ser Unit} {get : ByteArray → Nat → α} (n : Nat)
    (enc : α → List UInt8) (hlen : ∀ x, (enc x).length = n)
    (hser : ∀ x, (ser x).Writes d (enc x).toByteArray)
    (hget : ∀ b pos, pos + n ≤ b.size → (enc (get b pos)).toByteArray = b.extract pos (pos + n))
    (hdec : ∀ x b pos, b.extract pos (pos + n) = (enc x).toByteArray → get b pos = x) :
    Correct d ser (do let p ← De.advance n; return get (← De.input) p) ∧ Injective d ser := by
  refine ⟨⟨fun x _ => (hser x).uniform, ?_, ?_⟩, ?_⟩
  · intro x e _ he input pos hsize _
    rw [(hser x).unique he |>.symm] at hsize ⊢
    refine ⟨get input pos, De.fixed_eq_ok.mpr ⟨?_, rfl, ?_⟩⟩
    · simpa [List.size_toByteArray, hlen] using hsize
    · simp [List.size_toByteArray, hlen]
  · intro input pos y pos' h
    obtain ⟨hle, rfl, rfl⟩ := De.fixed_eq_ok.mp h
    refine ⟨trivial, by omega, ?_⟩
    rw [← hget _ _ hle]
    exact hser _
  · intro x y e _ _ hx hy
    have := (hser x).unique hx
    have := (hser y).unique hy
    have hxy : (enc x).toByteArray = (enc y).toByteArray := by simp_all
    have hext : (enc y).toByteArray.extract 0 (0 + n) = (enc y).toByteArray := by
      rw [Nat.zero_add, ← hlen y, ← List.size_toByteArray, ByteArray.extract_zero_size]
    rw [← hdec x _ 0 (hext.trans hxy.symm), hdec y _ 0 hext]

def enc16 (x : UInt16) : List UInt8 := [x.toUInt8, (x >>> 8).toUInt8]

def enc32 (x : UInt32) : List UInt8 :=
  [x.toUInt8, (x >>> 8).toUInt8, (x >>> 16).toUInt8, (x >>> 24).toUInt8]

def enc64 (x : UInt64) : List UInt8 := enc32 x.toUInt32 ++ enc32 (x >>> 32).toUInt32

def enc128 (x : UInt128) : List UInt8 := enc64 x.lo ++ enc64 x.hi

theorem push_push {b : ByteArray} : (b.push x).push y = b ++ [x, y].toByteArray := by
  rw [← ByteArray.append_toByteArray_singleton, ← ByteArray.append_toByteArray_singleton,
    ByteArray.append_assoc, ← List.toByteArray_append]
  rfl

theorem pushUInt16LE_eq : Bytes.pushUInt16LE b x = b ++ (enc16 x).toByteArray := push_push

theorem pushUInt32LE_eq : Bytes.pushUInt32LE b x = b ++ (enc32 x).toByteArray := by
  simp only [Bytes.pushUInt32LE, ← ByteArray.append_toByteArray_singleton, ByteArray.append_assoc,
    ← List.toByteArray_append]
  rfl

theorem pushUInt64LE_eq : Bytes.pushUInt64LE b x = b ++ (enc64 x).toByteArray := by
  simp only [Bytes.pushUInt64LE, pushUInt32LE_eq, enc64, ByteArray.append_assoc,
    List.toByteArray_append]

theorem pushUInt128LE_eq : Bytes.pushUInt128LE b x = b ++ (enc128 x).toByteArray := by
  simp only [Bytes.pushUInt128LE, pushUInt64LE_eq, enc128, ByteArray.append_assoc,
    List.toByteArray_append]

/-! Arithmetic facts on little-endian encodings, proved with `omega`. -/

theorem enc16_get (a b : UInt8) : enc16 (a.toUInt16 + b.toUInt16 * 0x100) = [a, b] := by
  have := a.toNat_lt; have := b.toNat_lt
  simp only [enc16, List.cons.injEq, and_true]
  constructor <;> apply UInt8.toNat_inj.mp <;>
    simp [UInt16.toNat_add, UInt16.toNat_mul, Nat.shiftRight_eq_div_pow] <;> omega

theorem get_enc16 (x : UInt16) :
    x.toUInt8.toUInt16 + (x >>> 8).toUInt8.toUInt16 * (0x100 : UInt16) = x := by
  have := x.toNat_lt
  apply UInt16.toNat_inj.mp
  simp [UInt16.toNat_add, UInt16.toNat_mul, Nat.shiftRight_eq_div_pow]
  omega

theorem enc32_get (a b c d : UInt8) :
    enc32 (a.toUInt32 + b.toUInt32 * 0x100 + c.toUInt32 * 0x10000 + d.toUInt32 * 0x1000000) =
      [a, b, c, d] := by
  have := a.toNat_lt; have := b.toNat_lt; have := c.toNat_lt; have := d.toNat_lt
  simp only [enc32, List.cons.injEq, and_true]
  refine ⟨?_, ?_, ?_, ?_⟩ <;> apply UInt8.toNat_inj.mp <;>
    simp [UInt32.toNat_add, UInt32.toNat_mul, Nat.shiftRight_eq_div_pow] <;> omega

theorem get_enc32 (x : UInt32) :
    x.toUInt8.toUInt32 + (x >>> 8).toUInt8.toUInt32 * (0x100 : UInt32) +
      (x >>> 16).toUInt8.toUInt32 * (0x10000 : UInt32) +
      (x >>> 24).toUInt8.toUInt32 * (0x1000000 : UInt32) = x := by
  have := x.toNat_lt
  apply UInt32.toNat_inj.mp
  simp [UInt32.toNat_add, UInt32.toNat_mul, Nat.shiftRight_eq_div_pow]
  omega

theorem split64 (u v : UInt32) :
    (u.toUInt64 + v.toUInt64 * 0x100000000).toUInt32 = u ∧
      ((u.toUInt64 + v.toUInt64 * 0x100000000) >>> 32).toUInt32 = v := by
  have := u.toNat_lt; have := v.toNat_lt
  constructor <;> apply UInt32.toNat_inj.mp <;>
    simp [UInt64.toNat_add, UInt64.toNat_mul, Nat.shiftRight_eq_div_pow] <;> omega

theorem join64 (x : UInt64) :
    x.toUInt32.toUInt64 + (x >>> 32).toUInt32.toUInt64 * (0x100000000 : UInt64) = x := by
  have := x.toNat_lt
  apply UInt64.toNat_inj.mp
  simp [UInt64.toNat_add, UInt64.toNat_mul, Nat.shiftRight_eq_div_pow]
  omega

/-! Reading and writing fixed-width integers. -/

theorem extract_add {b : ByteArray} {pos m n : Nat} :
    b.extract pos (pos + (m + n)) = b.extract pos (pos + m) ++ b.extract (pos + m) (pos + m + n) := by
  rw [extract_concat (by omega) (by omega), Nat.add_assoc]

theorem extract_split' {b : ByteArray} {l₁ l₂ : List UInt8} {pos : Nat}
    (h : b.extract pos (pos + (l₁.length + l₂.length)) = (l₁ ++ l₂).toByteArray) :
    b.extract pos (pos + l₁.length) = l₁.toByteArray ∧
      b.extract (pos + l₁.length) (pos + l₁.length + l₂.length) = l₂.toByteArray := by
  rw [List.toByteArray_append] at h
  have := extract_split (input := b) (pos := pos) (e₁ := l₁.toByteArray) (e₂ := l₂.toByteArray)
  simp only [ByteArray.size_append, List.size_toByteArray] at this
  exact this h

theorem get16_spec {b : ByteArray} (h : pos + 2 ≤ b.size) :
    (enc16 (Bytes.getUInt16LE b pos)).toByteArray = b.extract pos (pos + 2) := by
  simp only [Bytes.getUInt16LE, ByteArray.get!_eq_getElem!]
  rw [extract_eq_ofFn h]
  simp only [List.ofFn_succ, List.ofFn_zero, Fin.val_zero, Fin.val_succ, Nat.add_zero]
  exact congrArg List.toByteArray (enc16_get _ _)

theorem dec16_spec {b : ByteArray} (h : b.extract pos (pos + 2) = (enc16 x).toByteArray) :
    Bytes.getUInt16LE b pos = x := by
  have h := getElem!_of_extract (l := enc16 x) h
  have h₀ := h 0 (by simp [enc16])
  have h₁ := h 1 (by simp [enc16])
  rw [Nat.add_zero] at h₀
  simp only [Bytes.getUInt16LE, ByteArray.get!_eq_getElem!, h₀, h₁]
  exact get_enc16 x

theorem get32_spec {b : ByteArray} (h : pos + 4 ≤ b.size) :
    (enc32 (Bytes.getUInt32LE b pos)).toByteArray = b.extract pos (pos + 4) := by
  simp only [Bytes.getUInt32LE, ByteArray.get!_eq_getElem!]
  rw [extract_eq_ofFn h]
  simp only [List.ofFn_succ, List.ofFn_zero, Fin.val_zero, Fin.val_succ, Nat.add_zero]
  exact congrArg List.toByteArray (enc32_get _ _ _ _)

theorem dec32_spec {b : ByteArray} (h : b.extract pos (pos + 4) = (enc32 x).toByteArray) :
    Bytes.getUInt32LE b pos = x := by
  have h := getElem!_of_extract (l := enc32 x) h
  have h₀ := h 0 (by simp [enc32])
  have h₁ := h 1 (by simp [enc32])
  have h₂ := h 2 (by simp [enc32])
  have h₃ := h 3 (by simp [enc32])
  rw [Nat.add_zero] at h₀
  simp only [Bytes.getUInt32LE, ByteArray.get!_eq_getElem!, h₀, h₁, h₂, h₃]
  exact get_enc32 x

theorem get64_spec {b : ByteArray} (h : pos + 8 ≤ b.size) :
    (enc64 (Bytes.getUInt64LE b pos)).toByteArray = b.extract pos (pos + 8) := by
  simp only [enc64, Bytes.getUInt64LE, split64, List.toByteArray_append]
  rw [get32_spec (by omega), get32_spec (by omega), show pos + 8 = pos + (4 + 4) from rfl,
    extract_add]

theorem dec64_spec {b : ByteArray} (h : b.extract pos (pos + 8) = (enc64 x).toByteArray) :
    Bytes.getUInt64LE b pos = x := by
  obtain ⟨h₁, h₂⟩ := extract_split' (l₁ := enc32 x.toUInt32) (l₂ := enc32 (x >>> 32).toUInt32) h
  simp only [enc32, List.length_cons, List.length_nil] at h₁ h₂
  simp only [Bytes.getUInt64LE, dec32_spec h₁, dec32_spec h₂]
  exact join64 x

theorem get128_spec {b : ByteArray} (h : pos + 16 ≤ b.size) :
    (enc128 (Bytes.getUInt128LE b pos)).toByteArray = b.extract pos (pos + 16) := by
  simp only [enc128, Bytes.getUInt128LE, UInt128.lo_ofHalves, UInt128.hi_ofHalves,
    List.toByteArray_append]
  rw [get64_spec (by omega), get64_spec (by omega), show pos + 16 = pos + (8 + 8) from rfl,
    extract_add]

theorem dec128_spec {b : ByteArray} (h : b.extract pos (pos + 16) = (enc128 x).toByteArray) :
    Bytes.getUInt128LE b pos = x := by
  obtain ⟨h₁, h₂⟩ := extract_split' (l₁ := enc64 x.lo) (l₂ := enc64 x.hi) h
  simp only [enc64, enc32, List.length_append, List.length_cons, List.length_nil] at h₁ h₂
  simp only [Bytes.getUInt128LE, dec64_spec h₁, dec64_spec h₂]
  exact UInt128.ofHalves_lo_hi x

theorem correct_u8 : Correct d Ser.u8 De.u8 ∧ Injective d Ser.u8 :=
  correct_fixed 1 (fun x => [x]) (fun _ => rfl) (fun _ => Ser.writes_u8)
    (fun b pos h => by rw [extract_eq_ofFn h]; simp [ByteArray.get!_eq_getElem!])
    (fun x b pos h => by
      simpa [ByteArray.get!_eq_getElem!] using getElem!_of_extract (l := [x]) h 0 (by simp))

theorem correct_u16 : Correct d Ser.u16 De.u16 ∧ Injective d Ser.u16 :=
  correct_fixed 2 enc16 (fun _ => rfl) (fun _ => Ser.writes_modify fun _ => pushUInt16LE_eq)
    (fun _ _ => get16_spec) (fun _ _ _ => dec16_spec)

theorem correct_u32 : Correct d Ser.u32 De.u32 ∧ Injective d Ser.u32 :=
  correct_fixed 4 enc32 (fun _ => rfl) (fun _ => Ser.writes_modify fun _ => pushUInt32LE_eq)
    (fun _ _ => get32_spec) (fun _ _ _ => dec32_spec)

theorem correct_u64 : Correct d Ser.u64 De.u64 ∧ Injective d Ser.u64 :=
  correct_fixed 8 enc64 (fun _ => by simp [enc64, enc32])
    (fun _ => Ser.writes_modify fun _ => pushUInt64LE_eq)
    (fun _ _ => get64_spec) (fun _ _ _ => dec64_spec)

theorem correct_u128 : Correct d Ser.u128 De.u128 ∧ Injective d Ser.u128 :=
  correct_fixed 16 enc128 (fun _ => by simp [enc128, enc64, enc32])
    (fun _ => Ser.writes_modify fun _ => pushUInt128LE_eq)
    (fun _ _ => get128_spec) (fun _ _ _ => dec128_spec)

end Serde
