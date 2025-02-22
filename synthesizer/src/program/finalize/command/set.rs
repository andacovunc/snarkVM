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

use crate::{Load, Opcode, Operand, ProgramStorage, ProgramStore, Stack};
use console::{
    network::prelude::*,
    program::{Identifier, Value},
};

/// A set command, e.g. `set r1 into mapping[r0];`
/// Sets the `key` entry as `value` in `mapping`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Set<N: Network> {
    /// The mapping name.
    mapping: Identifier<N>,
    /// The key to access the mapping.
    key: Operand<N>,
    /// The value to be set.
    value: Operand<N>,
}

impl<N: Network> Set<N> {
    /// Returns the opcode.
    #[inline]
    pub const fn opcode() -> Opcode {
        Opcode::Command("set")
    }

    /// Returns the operands in the operation.
    #[inline]
    pub fn operands(&self) -> Vec<Operand<N>> {
        vec![self.value.clone(), self.key.clone()]
    }

    /// Returns the mapping name.
    #[inline]
    pub const fn mapping_name(&self) -> &Identifier<N> {
        &self.mapping
    }

    /// Returns the operand containing the key.
    #[inline]
    pub const fn key(&self) -> &Operand<N> {
        &self.key
    }

    /// Returns the operand containing the value.
    #[inline]
    pub const fn value(&self) -> &Operand<N> {
        &self.value
    }
}

impl<N: Network> Set<N> {
    /// Finalizes the command.
    #[inline]
    pub fn finalize<P: ProgramStorage<N>>(
        &self,
        stack: &Stack<N>,
        store: &ProgramStore<N, P>,
        registers: &mut impl Load<N>,
    ) -> Result<()> {
        // Ensure the mapping exists in storage.
        if !store.contains_mapping(stack.program_id(), &self.mapping)? {
            bail!("Mapping '{}/{}' does not exist in storage", stack.program_id(), self.mapping);
        }

        // Load the key operand as a plaintext.
        let key = registers.load_plaintext(stack, &self.key)?;
        // Load the value operand as a plaintext.
        let value = Value::Plaintext(registers.load_plaintext(stack, &self.value)?);

        // Update the value in storage.
        store.update_key_value(stack.program_id(), &self.mapping, key, value)?;

        Ok(())
    }
}

impl<N: Network> Parser for Set<N> {
    /// Parses a string into an operation.
    #[inline]
    fn parse(string: &str) -> ParserResult<Self> {
        // Parse the whitespace and comments from the string.
        let (string, _) = Sanitizer::parse(string)?;
        // Parse the opcode from the string.
        let (string, _) = tag(*Self::opcode())(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;

        // Parse the value operand from the string.
        let (string, value) = Operand::parse(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;

        // Parse the "into" keyword from the string.
        let (string, _) = tag("into")(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;

        // Parse the mapping name from the string.
        let (string, mapping) = Identifier::parse(string)?;
        // Parse the "[" from the string.
        let (string, _) = tag("[")(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;
        // Parse the key operand from the string.
        let (string, key) = Operand::parse(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;
        // Parse the "]" from the string.
        let (string, _) = tag("]")(string)?;
        // Parse the whitespace from the string.
        let (string, _) = Sanitizer::parse_whitespaces(string)?;
        // Parse the ";" from the string.
        let (string, _) = tag(";")(string)?;

        Ok((string, Self { mapping, key, value }))
    }
}

impl<N: Network> FromStr for Set<N> {
    type Err = Error;

    /// Parses a string into the command.
    #[inline]
    fn from_str(string: &str) -> Result<Self> {
        match Self::parse(string) {
            Ok((remainder, object)) => {
                // Ensure the remainder is empty.
                ensure!(remainder.is_empty(), "Failed to parse string. Found invalid character in: \"{remainder}\"");
                // Return the object.
                Ok(object)
            }
            Err(error) => bail!("Failed to parse string. {error}"),
        }
    }
}

impl<N: Network> Debug for Set<N> {
    /// Prints the command as a string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<N: Network> Display for Set<N> {
    /// Prints the command to a string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Print the command.
        write!(f, "{} ", Self::opcode())?;
        // Print the value operand.
        write!(f, "{} into ", self.value)?;
        // Print the mapping and key operand.
        write!(f, "{}[{}];", self.mapping, self.key)
    }
}

impl<N: Network> FromBytes for Set<N> {
    /// Reads the command from a buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the mapping name.
        let mapping = Identifier::read_le(&mut reader)?;
        // Read the key operand.
        let key = Operand::read_le(&mut reader)?;
        // Read the value operand.
        let value = Operand::read_le(&mut reader)?;
        // Return the command.
        Ok(Self { mapping, key, value })
    }
}

impl<N: Network> ToBytes for Set<N> {
    /// Writes the operation to a buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the mapping name.
        self.mapping.write_le(&mut writer)?;
        // Write the key operand.
        self.key.write_le(&mut writer)?;
        // Write the value operand.
        self.value.write_le(&mut writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::{network::Testnet3, program::Register};

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_parse() {
        let (string, set) = Set::<CurrentNetwork>::parse("set r0 into account[r1];").unwrap();
        assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");
        assert_eq!(set.mapping, Identifier::from_str("account").unwrap());
        assert_eq!(set.operands().len(), 2, "The number of operands is incorrect");
        assert_eq!(set.value, Operand::Register(Register::Locator(0)), "The first operand is incorrect");
        assert_eq!(set.key, Operand::Register(Register::Locator(1)), "The second operand is incorrect");
    }
}
