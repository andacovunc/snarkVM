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

use super::*;

impl<A: Aleo, Private: Visibility<A>> Equal<Self> for Record<A, Private> {
    type Output = Boolean<A>;

    /// Returns `true` if `self` and `other` are equal.
    ///
    /// Note: This method does **not** check the `nonce` equality.
    fn is_equal(&self, other: &Self) -> Self::Output {
        // Recursively check each entry for equality.
        let mut equal = Boolean::constant(true);
        for ((name_a, entry_a), (name_b, entry_b)) in self.data.iter().zip_eq(other.data.iter()) {
            equal = equal & name_a.is_equal(name_b) & entry_a.is_equal(entry_b);
        }

        // Check the `owner`, `data`, and `nonce`.
        self.owner.is_equal(&other.owner) & equal & self.nonce.is_equal(&other.nonce)
    }

    /// Returns `true` if `self` and `other` are *not* equal.
    ///
    /// Note: This method does **not** check the `nonce` equality.
    fn is_not_equal(&self, other: &Self) -> Self::Output {
        // Recursively check each entry for inequality.
        let mut not_equal = Boolean::constant(false);
        for ((name_a, entry_a), (name_b, entry_b)) in self.data.iter().zip_eq(other.data.iter()) {
            not_equal = not_equal | name_a.is_not_equal(name_b) | entry_a.is_not_equal(entry_b);
        }

        // Check the `owner`, `data`, and `nonce`.
        self.owner.is_not_equal(&other.owner) | not_equal | self.nonce.is_not_equal(&other.nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Circuit;

    fn sample_record(mode: Mode) -> Record<Circuit, Plaintext<Circuit>> {
        let record = console::Record::<
            <Circuit as Environment>::Network,
            console::Plaintext<<Circuit as Environment>::Network>,
        >::from_str(
            r"{
    owner: aleo14tlamssdmg3d0p5zmljma573jghe2q9n6wz29qf36re2glcedcpqfg4add.private,
    a: true.private,
    b: 123456789field.public,
    c: 0group.private,
    d: {
        e: true.private,
        f: 123456789field.private,
        g: 0group.private
    },
    _nonce: 0group.public
}",
        )
        .unwrap();
        Record::new(mode, record)
    }

    fn sample_mismatched_record(mode: Mode) -> Record<Circuit, Plaintext<Circuit>> {
        let record = console::Record::<
            <Circuit as Environment>::Network,
            console::Plaintext<<Circuit as Environment>::Network>,
        >::from_str(
            r"{
    owner: aleo14tlamssdmg3d0p5zmljma573jghe2q9n6wz29qf36re2glcedcpqfg4add.private,
    a: true.public,
    b: 123456789field.public,
    c: 0group.private,
    d: {
        e: true.private,
        f: 123456789field.private,
        g: 0group.private
    },
    _nonce: 0group.public
}",
        )
        .unwrap();
        Record::new(mode, record)
    }

    fn check_is_equal(
        mode: Mode,
        num_constants: u64,
        num_public: u64,
        num_private: u64,
        num_constraints: u64,
    ) -> Result<()> {
        // Sample the record.
        let record = sample_record(mode);
        let mismatched_record = sample_mismatched_record(mode);

        Circuit::scope(format!("{mode}"), || {
            let candidate = record.is_equal(&record);
            assert!(candidate.eject_value());
            assert_scope!(num_constants, num_public, num_private, num_constraints);
        });

        Circuit::scope(format!("{mode}"), || {
            let candidate = record.is_equal(&mismatched_record);
            assert!(!candidate.eject_value());
        });

        Circuit::reset();
        Ok(())
    }

    fn check_is_not_equal(
        mode: Mode,
        num_constants: u64,
        num_public: u64,
        num_private: u64,
        num_constraints: u64,
    ) -> Result<()> {
        // Sample the record.
        let record = sample_record(mode);
        let mismatched_record = sample_mismatched_record(mode);

        Circuit::scope(format!("{mode}"), || {
            let candidate = record.is_not_equal(&mismatched_record);
            assert!(candidate.eject_value());
            assert_scope!(num_constants, num_public, num_private, num_constraints);
        });

        Circuit::scope(format!("{mode}"), || {
            let candidate = record.is_not_equal(&record);
            assert!(!candidate.eject_value());
        });

        Circuit::reset();
        Ok(())
    }

    #[test]
    fn test_is_equal_constant() -> Result<()> {
        // Note: This is correct. At this (high) level of a program, we override the default mode in the `Record` case,
        // based on the user-defined visibility in the record type. Thus, we have nonzero private and constraint values.
        check_is_equal(Mode::Constant, 7, 0, 33, 43)
    }

    #[test]
    fn test_is_equal_public() -> Result<()> {
        check_is_equal(Mode::Public, 7, 0, 33, 43)
    }

    #[test]
    fn test_is_equal_private() -> Result<()> {
        check_is_equal(Mode::Private, 7, 0, 33, 43)
    }

    #[test]
    fn test_is_not_equal_constant() -> Result<()> {
        // Note: This is correct. At this (high) level of a program, we override the default mode in the `Record` case,
        // based on the user-defined visibility in the record type. Thus, we have nonzero private and constraint values.
        check_is_not_equal(Mode::Constant, 7, 0, 27, 37)
    }

    #[test]
    fn test_is_not_equal_public() -> Result<()> {
        check_is_not_equal(Mode::Public, 7, 0, 27, 37)
    }

    #[test]
    fn test_is_not_equal_private() -> Result<()> {
        check_is_not_equal(Mode::Private, 7, 0, 27, 37)
    }
}
