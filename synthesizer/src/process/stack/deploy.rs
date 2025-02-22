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

impl<N: Network> Stack<N> {
    /// Deploys the given program ID, if it does not exist.
    #[inline]
    pub fn deploy<A: circuit::Aleo<Network = N>, R: Rng + CryptoRng>(&self, rng: &mut R) -> Result<Deployment<N>> {
        let timer = timer!("Stack::deploy");

        // Ensure the program contains functions.
        ensure!(!self.program.functions().is_empty(), "Program '{}' has no functions", self.program.id());

        // Initialize a vector for the verifying keys and certificates.
        let mut verifying_keys = Vec::with_capacity(self.program.functions().len());

        for function_name in self.program.functions().keys() {
            // Synthesize the proving and verifying key.
            self.synthesize_key::<A, R>(function_name, rng)?;
            lap!(timer, "Synthesize key for {function_name}");

            // Retrieve the proving key.
            let proving_key = self.get_proving_key(function_name)?;
            // Retrieve the verifying key.
            let verifying_key = self.get_verifying_key(function_name)?;
            lap!(timer, "Retrieve the keys for {function_name}");

            // Certify the circuit.
            let certificate = Certificate::certify(function_name, &proving_key, &verifying_key)?;
            lap!(timer, "Certify the circuit");

            // Add the verifying key and certificate to the bundle.
            verifying_keys.push((*function_name, (verifying_key, certificate)));
        }

        finish!(timer);

        // Return the deployment.
        Deployment::new(N::EDITION, self.program.clone(), verifying_keys)
    }

    /// Checks each function in the program on the given verifying key and certificate.
    #[inline]
    pub fn verify_deployment<A: circuit::Aleo<Network = N>, R: Rng + CryptoRng>(
        &self,
        deployment: &Deployment<N>,
        rng: &mut R,
    ) -> Result<()> {
        let timer = timer!("Stack::verify_deployment");

        // Sanity Checks //

        // Ensure the deployment is ordered.
        deployment.check_is_ordered()?;
        // Ensure the program in the stack and deployment matches.
        ensure!(&self.program == deployment.program(), "The stack program does not match the deployment program");

        // Check Verifying Keys //

        let program_id = self.program.id();

        // Iterate through the program functions.
        for (function, (_, (verifying_key, certificate))) in
            deployment.program().functions().values().zip_eq(deployment.verifying_keys())
        {
            // Initialize a burner private key.
            let burner_private_key = PrivateKey::new(rng)?;
            // Compute the burner address.
            let burner_address = Address::try_from(&burner_private_key)?;
            // Retrieve the input types.
            let input_types = function.input_types();
            // Sample the inputs.
            let inputs = input_types
                .iter()
                .map(|input_type| match input_type {
                    ValueType::ExternalRecord(locator) => {
                        // Retrieve the external stack.
                        let stack = self.get_external_stack(locator.program_id())?;
                        // Sample the input.
                        stack.sample_value(&burner_address, &ValueType::Record(*locator.resource()), rng)
                    }
                    _ => self.sample_value(&burner_address, input_type, rng),
                })
                .collect::<Result<Vec<_>>>()?;
            lap!(timer, "Sample the inputs");

            // Compute the request, with a burner private key.
            let request = Request::sign(
                &burner_private_key,
                *program_id,
                *function.name(),
                inputs.into_iter(),
                &input_types,
                rng,
            )?;
            lap!(timer, "Compute the request for {}", function.name());
            // Initialize the assignments.
            let assignments = Assignments::<N>::default();
            // Initialize the call stack.
            let call_stack = CallStack::CheckDeployment(vec![request], burner_private_key, assignments.clone());
            // Synthesize the circuit.
            let _response = self.execute_function::<A, R>(call_stack, rng)?;
            lap!(timer, "Synthesize the circuit");
            // Check the certificate.
            match assignments.read().last() {
                None => {
                    bail!("The assignment for function '{}' is missing in '{program_id}'", function.name())
                }
                Some(assignment) => {
                    // Ensure the certificate is valid.
                    if !certificate.verify(function.name(), assignment, verifying_key) {
                        bail!("The certificate for function '{}' is invalid in '{program_id}'", function.name())
                    }
                    lap!(timer, "Ensure the certificate is valid");
                }
            };
        }

        finish!(timer);

        Ok(())
    }
}
