// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

#![allow(clippy::module_inception)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate thiserror;

#[macro_use]
mod macros;

pub mod errors;
pub use errors::*;

mod fp_256;
pub use fp_256::*;

mod fp_384;
pub use fp_384::*;

mod fp2;
pub use fp2::*;

pub mod fp6_3over2;

mod fp12_2over3over2;
pub use fp12_2over3over2::*;

mod legendre;
pub use legendre::*;

mod to_field_vec;
pub use to_field_vec::*;

pub mod traits;
pub use traits::*;

use snarkvm_utilities::{
    biginteger::*,
    serialize::{CanonicalDeserialize, CanonicalDeserializeWithFlags, CanonicalSerialize, CanonicalSerializeWithFlags},
    FromBytes,
    ToBytes,
};

impl_field_to_biginteger!(Fp256, BigInteger256, Fp256Parameters);
impl_field_to_biginteger!(Fp384, BigInteger384, Fp384Parameters);

impl_primefield_serializer!(Fp256, Fp256Parameters, 32);
impl_primefield_serializer!(Fp384, Fp384Parameters, 48);

// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
pub fn batch_inversion<F: Field>(v: &mut [F]) {
    batch_inversion_and_mul(v, &F::one());
}

#[cfg(feature = "serial")]
// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
pub fn batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    serial_batch_inversion_and_mul(v, coeff);
}

#[cfg(not(feature = "serial"))]
// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
pub fn batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    use rayon::prelude::*;
    // Divide the vector v evenly between all available cores
    let min_elements_per_thread = 1;
    let num_cpus_available = snarkvm_utilities::parallel::max_available_threads();
    let num_elems = v.len();
    let num_elem_per_thread = min_elements_per_thread.max(num_elems / num_cpus_available);

    // Batch invert in parallel, without copying the vector
    v.par_chunks_mut(num_elem_per_thread).for_each(|chunk| {
        serial_batch_inversion_and_mul(chunk, coeff);
    });
}

/// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}.
/// This method is explicitly single-threaded.
fn serial_batch_inversion_and_mul<F: Field>(v: &mut [F], coeff: &F) {
    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by
    // coeff

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = F::one();
    for f in v.iter().filter(|f| !f.is_zero()) {
        tmp.mul_assign(f);
        prod.push(tmp);
    }

    // Invert `tmp`.
    tmp = tmp.inverse().unwrap(); // Guaranteed to be nonzero.

    // Multiply product by coeff, so all inverses will be scaled by coeff
    tmp *= coeff;

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v.iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(F::one())))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * s;
        tmp = new_tmp;
    }
}
