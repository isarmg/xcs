// Copyright 2016 Brian Smith.
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
// SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION
// OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN
// CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use crate::{
    arithmetic::limbs_from_hex,
    arithmetic::montgomery::*,
    bb::LeakyWord,
    cpu,
    error::{self, LenMismatchError},
    limb::*,
};
use core::marker::PhantomData;

use elem::{mul_mont, unary_op, unary_op_assign, unary_op_from_binary_op_assign};

/// A field element, i.e. an element of ℤ/qℤ for the curve's field modulus
/// *q*.
pub type Elem<E> = elem::Elem<Q, E>;
type PublicElem<E> = elem::PublicElem<Q, E>;

/// Represents the (prime) order *q* of the curve's prime field.
#[derive(Clone, Copy)]
pub enum Q {}

/// A scalar. Its value is in [0, n). Zero-valued scalars are forbidden in most
/// contexts.
pub type Scalar<E = Unencoded> = elem::Elem<N, E>;
type PublicScalar<E> = elem::PublicElem<N, E>;

/// Represents the prime order *n* of the curve's group.
#[derive(Clone, Copy)]
pub enum N {}

pub(super) struct Modulus<M> {
    // TODO: [Limb; elem::NumLimbs::MAX]
    limbs: &'static [Limb; elem::NumLimbs::MAX],
    num_limbs: elem::NumLimbs,
    cops: &'static CommonOps,
    m: PhantomData<M>,
    cpu: cpu::Features,
}

pub struct Point {
    // The coordinates are stored in a contiguous array, where the first
    // `ops.num_limbs` elements are the X coordinate, the next
    // `ops.num_limbs` elements are the Y coordinate, and the next
    // `ops.num_limbs` elements are the Z coordinate. This layout is dictated
    // by the requirements of the nistz256 code.
    xyz: [Limb; 3 * elem::NumLimbs::MAX],
}

impl Point {
    pub fn new_at_infinity() -> Self {
        Self {
            xyz: [0; 3 * elem::NumLimbs::MAX],
        }
    }
}

/// Operations and values needed by all curve operations.
pub struct CommonOps {
    num_limbs: elem::NumLimbs,
    q: PublicModulus,
    n: PublicElem<Unencoded>,

    pub a: PublicElem<R>, // Must be -3 mod q
    pub b: PublicElem<R>,

    // In all cases, `r`, `a`, and `b` may all alias each other.
    elem_mul_mont: unsafe extern "C" fn(r: *mut Limb, a: *const Limb, b: *const Limb),
    elem_sqr_mont: unsafe extern "C" fn(r: *mut Limb, a: *const Limb),
}

impl CommonOps {
    pub(super) fn elem_modulus(&'static self, cpu_features: cpu::Features) -> Modulus<Q> {
        Modulus {
            // TODO: limbs: self.q.p.map(Limb::from),
            limbs: &self.q.p,
            num_limbs: self.num_limbs,
            cops: self,
            m: PhantomData,
            cpu: cpu_features,
        }
    }

    pub(super) fn scalar_modulus(&'static self, cpu_features: cpu::Features) -> Modulus<N> {
        Modulus {
            // TODO: limbs: self.n.limbs.map(Limb::from),
            limbs: &self.n.limbs,
            num_limbs: self.num_limbs,
            cops: self,
            m: PhantomData,
            cpu: cpu_features,
        }
    }

    // The length of a field element, which is the same as the length of a
    // scalar, in bytes.
    pub fn len(&self) -> usize {
        // Keep in sync with `Modulus<M>::len()`
        self.num_limbs.into() * LIMB_BYTES
    }

    #[cfg(test)]
    pub(super) fn n_limbs(&self) -> &[Limb] {
        &self.n.limbs[..self.num_limbs.into()]
    }
}

impl<M> Modulus<M> {
    pub fn cpu(&self) -> cpu::Features {
        self.cpu
    }

    // Keep in sync with `CommonOps::len()`.
    pub fn bytes_len(&self) -> usize {
        self.num_limbs.into() * LIMB_BYTES
    }
}

impl<M> Modulus<M> {
    #[inline]
    pub fn add_assign<E: Encoding>(&self, a: &mut elem::Elem<M, E>, b: &elem::Elem<M, E>) {
        let num_limbs = self.num_limbs.into();
        limbs_add_assign_mod(
            &mut a.limbs[..num_limbs],
            &b.limbs[..num_limbs],
            &self.limbs[..num_limbs],
        )
        .unwrap_or_else(unwrap_impossible_len_mismatch_error)
    }
}

impl Modulus<Q> {
    #[inline]
    pub fn elems_are_equal<E: Encoding>(&self, a: &Elem<E>, b: &Elem<E>) -> LimbMask {
        let num_limbs = self.num_limbs.into();
        limbs_equal_limbs_consttime(&a.limbs[..num_limbs], &b.limbs[..num_limbs])
            .unwrap_or_else(unwrap_impossible_len_mismatch_error)
    }

    #[inline]
    pub fn elem_unencoded(&self, a: &Elem<R>) -> Elem<Unencoded> {
        self.elem_product(a, &Elem::one())
    }
}

impl CommonOps {
    #[inline]
    fn is_zero<M, E: Encoding>(&self, a: &elem::Elem<M, E>) -> bool {
        let num_limbs = self.num_limbs.into();
        limbs_are_zero(&a.limbs[..num_limbs]).leak()
    }

    #[inline]
    fn elem_mul(&self, a: &mut Elem<R>, b: &Elem<R>, _cpu: cpu::Features) {
        elem::binary_op_assign(self.elem_mul_mont, a, b)
    }

    #[inline]
    fn elem_product<EA: Encoding, EB: Encoding>(
        &self,
        a: &Elem<EA>,
        b: &Elem<EB>,
        _cpu: cpu::Features,
    ) -> Elem<<(EA, EB) as ProductEncoding>::Output>
    where
        (EA, EB): ProductEncoding,
    {
        mul_mont(self.elem_mul_mont, a, b)
    }

    #[inline]
    fn elem_square(&self, a: &mut Elem<R>, _cpu: cpu::Features) {
        unary_op_assign(self.elem_sqr_mont, a);
    }

    #[inline]
    fn elem_squared(&self, a: &Elem<R>, _cpu: cpu::Features) -> Elem<R> {
        unary_op(self.elem_sqr_mont, a)
    }
}

impl Modulus<Q> {
    #[inline]
    pub fn elem_mul(&self, a: &mut Elem<R>, b: &Elem<R>) {
        self.cops.elem_mul(a, b, self.cpu)
    }

    #[inline]
    pub fn elem_product<EA: Encoding, EB: Encoding>(
        &self,
        a: &Elem<EA>,
        b: &Elem<EB>,
    ) -> Elem<<(EA, EB) as ProductEncoding>::Output>
    where
        (EA, EB): ProductEncoding,
    {
        self.cops.elem_product(a, b, self.cpu)
    }

    #[inline]
    pub fn elem_square(&self, a: &mut Elem<R>) {
        self.cops.elem_square(a, self.cpu)
    }

    #[inline]
    pub fn elem_squared(&self, a: &Elem<R>) -> Elem<R> {
        self.cops.elem_squared(a, self.cpu)
    }
}

impl<M> Modulus<M> {
    #[inline]
    pub fn is_zero<E: Encoding>(&self, a: &elem::Elem<M, E>) -> bool {
        self.cops.is_zero(a)
    }
}

impl Modulus<Q> {
    pub fn elem_verify_is_not_zero(&self, a: &Elem<R>) -> Result<(), error::Unspecified> {
        if self.is_zero(a) {
            Err(error::Unspecified)
        } else {
            Ok(())
        }
    }

    pub(super) fn a(&self) -> &'static PublicElem<R> {
        &self.cops.a
    }
    pub(super) fn b(&self) -> &'static PublicElem<R> {
        &self.cops.b
    }
}

impl PrivateKeyOps {
    pub(super) fn point_sum(&self, a: &Point, b: &Point, _cpu: cpu::Features) -> Point {
        let mut r = Point::new_at_infinity();
        unsafe {
            (self.point_add_jacobian_impl)(r.xyz.as_mut_ptr(), a.xyz.as_ptr(), b.xyz.as_ptr())
        }
        r
    }
}

impl Modulus<Q> {
    pub fn point_x(&self, p: &Point) -> Elem<R> {
        let num_limbs = self.num_limbs.into();
        let mut r = Elem::zero();
        r.limbs[..num_limbs].copy_from_slice(&p.xyz[0..num_limbs]);
        r
    }

    pub fn point_y(&self, p: &Point) -> Elem<R> {
        let num_limbs = self.num_limbs.into();
        let mut r = Elem::zero();
        r.limbs[..num_limbs].copy_from_slice(&p.xyz[num_limbs..(2 * num_limbs)]);
        r
    }

    pub fn point_z(&self, p: &Point) -> Elem<R> {
        let num_limbs = self.num_limbs.into();
        let mut r = Elem::zero();
        r.limbs[..num_limbs].copy_from_slice(&p.xyz[(2 * num_limbs)..(3 * num_limbs)]);
        r
    }
}

struct PublicModulus {
    p: [LeakyLimb; elem::NumLimbs::MAX],
    rr: PublicElem<RR>,
}

/// Operations on private keys, for ECDH and ECDSA signing.
pub struct PrivateKeyOps {
    pub common: &'static CommonOps,
    elem_inv_squared: fn(q: &Modulus<Q>, a: &Elem<R>) -> Elem<R>,
    point_mul_base_impl: fn(a: &Scalar, cpu: cpu::Features) -> Point,
    point_mul_impl: unsafe extern "C" fn(
        r: *mut Limb,          // [3][num_limbs]
        p_scalar: *const Limb, // [num_limbs]
        p_x: *const Limb,      // [num_limbs]
        p_y: *const Limb,      // [num_limbs]
    ),
    point_add_jacobian_impl: unsafe extern "C" fn(r: *mut Limb, a: *const Limb, b: *const Limb),
}

impl PrivateKeyOps {
    pub fn leak_limbs<'a>(&self, a: &'a Elem<Unencoded>) -> &'a [Limb] {
        &a.limbs[..self.common.num_limbs.into()]
    }

    #[inline(always)]
    pub(super) fn point_mul_base(&self, a: &Scalar, cpu: cpu::Features) -> Point {
        (self.point_mul_base_impl)(a, cpu)
    }

    #[inline(always)]
    pub(super) fn point_mul(
        &self,
        p_scalar: &Scalar,
        (p_x, p_y): &(Elem<R>, Elem<R>),
        _cpu: cpu::Features,
    ) -> Point {
        let mut r = Point::new_at_infinity();
        unsafe {
            (self.point_mul_impl)(
                r.xyz.as_mut_ptr(),
                p_scalar.limbs.as_ptr(),
                p_x.limbs.as_ptr(),
                p_y.limbs.as_ptr(),
            );
        }
        r
    }

    #[inline]
    pub(super) fn elem_inverse_squared(&self, q: &Modulus<Q>, a: &Elem<R>) -> Elem<R> {
        (self.elem_inv_squared)(q, a)
    }
}

/// Operations and values needed by all operations on public keys (ECDH
/// agreement and ECDSA verification).
pub struct PublicKeyOps {
    pub common: &'static CommonOps,
}

impl PublicKeyOps {
    // The serialized bytes are in big-endian order, zero-padded. The limbs
    // of `Elem` are in the native endianness, least significant limb to
    // most significant limb. Besides the parsing, conversion, this also
    // implements NIST SP 800-56A Step 2: "Verify that xQ and yQ are integers
    // in the interval [0, p-1] in the case that q is an odd prime p[.]"
    pub(super) fn elem_parse(
        &self,
        q: &Modulus<Q>,
        input: &mut untrusted::Reader,
    ) -> Result<Elem<R>, error::Unspecified> {
        let _cpu = cpu::features();
        let encoded_value = input.read_bytes(self.common.len())?;
        let parsed = elem_parse_big_endian_fixed_consttime(q, encoded_value)?;
        let mut r = Elem::zero();
        let rr = Elem::from(&self.common.q.rr);
        // Montgomery encode (elem_to_mont).
        // TODO: do something about this.
        unsafe {
            (self.common.elem_mul_mont)(
                r.limbs.as_mut_ptr(),
                parsed.limbs.as_ptr(),
                rr.limbs.as_ptr(),
            )
        }
        Ok(r)
    }
}

// Operations used by both ECDSA signing and ECDSA verification. In general
// these must be side-channel resistant.
pub struct ScalarOps {
    pub common: &'static CommonOps,

    scalar_mul_mont: unsafe extern "C" fn(r: *mut Limb, a: *const Limb, b: *const Limb),
}

impl ScalarOps {
    pub(super) fn scalar_modulus(&'static self, cpu_features: cpu::Features) -> Modulus<N> {
        self.common.scalar_modulus(cpu_features)
    }

    // The (maximum) length of a scalar, not including any padding.
    pub fn scalar_bytes_len(&self) -> usize {
        self.common.len()
    }
}

impl ScalarOps {
    pub fn leak_limbs<'s>(&self, s: &'s Scalar) -> &'s [Limb] {
        &s.limbs[..self.common.num_limbs.into()]
    }

    #[inline]
    pub(super) fn scalar_product<EA: Encoding, EB: Encoding>(
        &self,
        a: &Scalar<EA>,
        b: &Scalar<EB>,
        _cpu: cpu::Features,
    ) -> Scalar<<(EA, EB) as ProductEncoding>::Output>
    where
        (EA, EB): ProductEncoding,
    {
        mul_mont(self.scalar_mul_mont, a, b)
    }
}

/// Operations on public scalars needed by ECDSA signature verification.
pub struct PublicScalarOps {
    pub scalar_ops: &'static ScalarOps,
    pub public_key_ops: &'static PublicKeyOps,

    pub(super) twin_mul: fn(
        g_scalar: &Scalar,
        p_scalar: &Scalar,
        p_xy: &(Elem<R>, Elem<R>),
        cpu: cpu::Features,
    ) -> Point,
    scalar_inv_to_mont_vartime: fn(s: &Scalar<Unencoded>, cpu: cpu::Features) -> Scalar<R>,
    pub(super) q_minus_n: PublicElem<Unencoded>,
}

impl PublicScalarOps {
    pub fn n(&self) -> &PublicElem<Unencoded> {
        &self.scalar_ops.common.n
    }

    #[inline]
    pub fn scalar_as_elem(&self, a: &Scalar) -> Elem<Unencoded> {
        Elem {
            limbs: a.limbs,
            m: PhantomData,
            encoding: PhantomData,
        }
    }
}

impl Modulus<Q> {
    pub fn elem_less_than_vartime(&self, a: &Elem<Unencoded>, b: &PublicElem<Unencoded>) -> bool {
        let num_limbs = self.num_limbs.into();
        limbs_less_than_limbs_vartime(&a.limbs[..num_limbs], &b.limbs[..num_limbs])
            .unwrap_or_else(|LenMismatchError { .. }| unreachable!())
    }
}

impl PublicScalarOps {
    pub(super) fn scalar_inv_to_mont_vartime(
        &self,
        s: &Scalar<Unencoded>,
        cpu: cpu::Features,
    ) -> Scalar<R> {
        (self.scalar_inv_to_mont_vartime)(s, cpu)
    }
}

#[allow(non_snake_case)]
pub struct PrivateScalarOps {
    pub scalar_ops: &'static ScalarOps,

    oneRR_mod_n: PublicScalar<RR>, // 1 * R**2 (mod n). TOOD: Use One<RR>.
    scalar_inv_to_mont: fn(a: Scalar<R>, cpu: cpu::Features) -> Scalar<R>,
}

impl PrivateScalarOps {
    pub(super) fn to_mont(&self, s: &Scalar<Unencoded>, cpu: cpu::Features) -> Scalar<R> {
        self.scalar_ops
            .scalar_product(s, &Scalar::from(&self.oneRR_mod_n), cpu)
    }

    /// Returns the modular inverse of `a` (mod `n`). Panics if `a` is zero.
    pub(super) fn scalar_inv_to_mont(&self, a: &Scalar, cpu: cpu::Features) -> Scalar<R> {
        assert!(!self.scalar_ops.common.is_zero(a));
        let a = self.to_mont(a, cpu);
        (self.scalar_inv_to_mont)(a, cpu)
    }
}

// XXX: Inefficient and unnecessarily depends on `PrivateKeyOps`. TODO: implement interleaved wNAF
// multiplication.
fn twin_mul_inefficient(
    ops: &PrivateKeyOps,
    g_scalar: &Scalar,
    p_scalar: &Scalar,
    p_xy: &(Elem<R>, Elem<R>),
    cpu: cpu::Features,
) -> Point {
    let scaled_g = ops.point_mul_base(g_scalar, cpu);
    let scaled_p = ops.point_mul(p_scalar, p_xy, cpu);
    ops.point_sum(&scaled_g, &scaled_p, cpu)
}

// This assumes n < q < 2*n.
impl Modulus<N> {
    pub fn elem_reduced_to_scalar(&self, elem: &Elem<Unencoded>) -> Scalar<Unencoded> {
        let num_limbs = self.num_limbs.into();
        let mut r_limbs = elem.limbs;
        limbs_reduce_once(&mut r_limbs[..num_limbs], &self.limbs[..num_limbs])
            .unwrap_or_else(unwrap_impossible_len_mismatch_error);
        Scalar {
            limbs: r_limbs,
            m: PhantomData,
            encoding: PhantomData,
        }
    }
}

// Returns (`a` squared `squarings` times) * `b`.
fn elem_sqr_mul(
    ops: &CommonOps,
    a: &Elem<R>,
    squarings: LeakyWord,
    b: &Elem<R>,
    cpu: cpu::Features,
) -> Elem<R> {
    debug_assert!(squarings >= 1);
    let mut tmp = ops.elem_squared(a, cpu);
    for _ in 1..squarings {
        ops.elem_square(&mut tmp, cpu);
    }
    ops.elem_product(&tmp, b, cpu)
}

// Sets `acc` = (`acc` squared `squarings` times) * `b`.
fn elem_sqr_mul_acc(
    ops: &CommonOps,
    acc: &mut Elem<R>,
    squarings: LeakyWord,
    b: &Elem<R>,
    cpu: cpu::Features,
) {
    debug_assert!(squarings >= 1);
    for _ in 0..squarings {
        ops.elem_square(acc, cpu);
    }
    ops.elem_mul(acc, b, cpu)
}

#[inline]
pub(super) fn elem_parse_big_endian_fixed_consttime(
    q: &Modulus<Q>,
    bytes: untrusted::Input,
) -> Result<Elem<Unencoded>, error::Unspecified> {
    parse_big_endian_fixed_consttime(q, bytes, AllowZero::Yes)
}

#[inline]
pub(super) fn scalar_parse_big_endian_fixed_consttime(
    n: &Modulus<N>,
    bytes: untrusted::Input,
) -> Result<Scalar, error::Unspecified> {
    parse_big_endian_fixed_consttime(n, bytes, AllowZero::No)
}

#[inline]
pub(super) fn scalar_parse_big_endian_variable(
    n: &Modulus<N>,
    allow_zero: AllowZero,
    bytes: untrusted::Input,
) -> Result<Scalar, error::Unspecified> {
    let num_limbs = n.num_limbs.into();
    let mut r = Scalar::zero();
    parse_big_endian_in_range_and_pad_consttime(
        bytes,
        allow_zero,
        &n.limbs[..num_limbs],
        &mut r.limbs[..num_limbs],
    )?;
    Ok(r)
}

pub(super) fn scalar_parse_big_endian_partially_reduced_variable_consttime(
    n: &Modulus<N>,
    bytes: untrusted::Input,
) -> Result<Scalar, error::Unspecified> {
    let num_limbs = n.num_limbs.into();
    let mut r = Scalar::zero();
    {
        let r = &mut r.limbs[..num_limbs];
        parse_big_endian_and_pad_consttime(bytes, r)?;
        limbs_reduce_once(r, &n.limbs[..num_limbs])
            .unwrap_or_else(unwrap_impossible_len_mismatch_error);
    }

    Ok(r)
}

fn parse_big_endian_fixed_consttime<M>(
    m: &Modulus<M>,
    bytes: untrusted::Input,
    allow_zero: AllowZero,
) -> Result<elem::Elem<M, Unencoded>, error::Unspecified> {
    let num_limbs = m.num_limbs.into();
    if bytes.len() != m.bytes_len() {
        return Err(error::Unspecified);
    }
    let mut r = elem::Elem::zero();
    parse_big_endian_in_range_and_pad_consttime(
        bytes,
        allow_zero,
        &m.limbs[..num_limbs],
        &mut r.limbs[..num_limbs],
    )?;
    Ok(r)
}

#[cold]
#[inline(never)]
fn unwrap_impossible_len_mismatch_error<T>(LenMismatchError { .. }: LenMismatchError) -> T {
    unreachable!()
}

#[cfg(test)]
include!("../../../../../../../test/foundation/ring/unit/ec/suite_b/ops/tests.rs");

mod elem;
pub mod p256;
pub mod p384;
