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

impl<N: Network> Record<N, Ciphertext<N>> {
    /// Decrypts `self` into plaintext using the given view key.
    pub fn decrypt(&self, view_key: &ViewKey<N>) -> Result<Record<N, Plaintext<N>>> {
        // Compute the record view key.
        let record_view_key = (self.nonce * **view_key).to_x_coordinate();
        // Decrypt the record.
        self.decrypt_symmetric(&record_view_key)
    }

    /// Decrypts `self` into plaintext using the given record view key.
    pub fn decrypt_symmetric(&self, record_view_key: &Field<N>) -> Result<Record<N, Plaintext<N>>> {
        // Determine the number of randomizers needed to encrypt the record.
        let num_randomizers = self.num_randomizers()?;
        // Prepare a randomizer for each field element.
        let randomizers = N::hash_many_psd8(&[N::encryption_domain(), *record_view_key], num_randomizers);
        // Decrypt the record.
        self.decrypt_with_randomizers(&randomizers)
    }

    /// Decrypts `self` into plaintext using the given randomizers.
    fn decrypt_with_randomizers(&self, randomizers: &[Field<N>]) -> Result<Record<N, Plaintext<N>>> {
        // Initialize an index to keep track of the randomizer index.
        let mut index: usize = 0;

        // Decrypt the owner.
        let owner = match self.owner.is_public() {
            true => self.owner.decrypt_with_randomizer(&[])?,
            false => self.owner.decrypt_with_randomizer(&[randomizers[index]])?,
        };

        // Increment the index if the owner is private.
        if owner.is_private() {
            index += 1;
        }

        // Decrypt the program data.
        let mut decrypted_data = IndexMap::with_capacity(self.data.len());
        for (id, entry, num_randomizers) in self.data.iter().map(|(id, entry)| (id, entry, entry.num_randomizers())) {
            // Retrieve the result for `num_randomizers`.
            let num_randomizers = num_randomizers? as usize;
            // Retrieve the randomizers for this entry.
            let randomizers = &randomizers[index..index + num_randomizers];
            // Decrypt the entry.
            let entry = match entry {
                // Constant entries do not need to be decrypted.
                Entry::Constant(plaintext) => Entry::Constant(plaintext.clone()),
                // Public entries do not need to be decrypted.
                Entry::Public(plaintext) => Entry::Public(plaintext.clone()),
                // Private entries are decrypted with the given randomizers.
                Entry::Private(private) => Entry::Private(Plaintext::from_fields(
                    &private
                        .iter()
                        .zip_eq(randomizers)
                        .map(|(ciphertext, randomizer)| *ciphertext - randomizer)
                        .collect::<Vec<_>>(),
                )?),
            };
            // Insert the decrypted entry.
            if decrypted_data.insert(*id, entry).is_some() {
                bail!("Duplicate identifier in record: {}", id);
            }
            // Increment the index.
            index += num_randomizers;
        }

        // Return the decrypted record.
        Self::from_plaintext(owner, decrypted_data, self.nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Literal;
    use snarkvm_console_account::PrivateKey;
    use snarkvm_console_network::Testnet3;
    use snarkvm_console_types::Field;

    type CurrentNetwork = Testnet3;

    const ITERATIONS: u64 = 100;

    fn check_encrypt_and_decrypt<N: Network>(
        view_key: ViewKey<N>,
        owner: Owner<N, Plaintext<N>>,
        rng: &mut TestRng,
    ) -> Result<()> {
        // Prepare the record.
        let randomizer = Scalar::rand(rng);
        let record = Record {
            owner,
            data: IndexMap::from_iter(
                vec![
                    (Identifier::from_str("a")?, Entry::Private(Plaintext::from(Literal::Field(Field::rand(rng))))),
                    (Identifier::from_str("b")?, Entry::Private(Plaintext::from(Literal::Scalar(Scalar::rand(rng))))),
                ]
                .into_iter(),
            ),
            nonce: N::g_scalar_multiply(&randomizer),
        };
        // Encrypt the record.
        let ciphertext = record.encrypt(randomizer)?;
        // Decrypt the record.
        assert_eq!(record, ciphertext.decrypt(&view_key)?);
        Ok(())
    }

    #[test]
    fn test_encrypt_and_decrypt() -> Result<()> {
        let mut rng = TestRng::default();

        for _ in 0..ITERATIONS {
            // Sample a view key and address.
            let private_key = PrivateKey::<CurrentNetwork>::new(&mut rng)?;
            let view_key = ViewKey::try_from(&private_key)?;
            let address = Address::try_from(&private_key)?;

            // Public owner.
            let owner = Owner::Public(address);
            check_encrypt_and_decrypt::<CurrentNetwork>(view_key, owner, &mut rng)?;

            // Private owner.
            let owner = Owner::Private(Plaintext::from(Literal::Address(address)));
            check_encrypt_and_decrypt::<CurrentNetwork>(view_key, owner, &mut rng)?;
        }
        Ok(())
    }
}
