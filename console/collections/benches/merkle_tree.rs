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

#[macro_use]
extern crate criterion;

use snarkvm_console_network::{
    prelude::{TestRng, ToBits, Uniform},
    Network,
    Testnet3,
};
use snarkvm_console_types::Field;

use criterion::{BatchSize, BenchmarkId, Criterion};

const DEPTH: u8 = 32;
const MAX_INSTANTIATED_DEPTH: u8 = 16;

const NUM_LEAVES: &[usize] = &[1, 10, 100, 1_000, 10_000, 100_000, 1_000_000];
const APPEND_SIZES: &[usize] = &[1, 10, 100, 1_000, 10_000, 100_000, 1_000_000];
const UPDATE_SIZES: &[usize] = &[1, 10, 100, 1_000, 10_000, 100_000, 1_000_000];

/// Generates the specified number of random Merkle tree leaves.
macro_rules! generate_leaves {
    ($num_leaves:expr, $rng:expr) => {{ (0..$num_leaves).map(|_| Field::<Testnet3>::rand($rng).to_bits_le()).collect::<Vec<_>>() }};
}

fn new(c: &mut Criterion) {
    let mut rng = TestRng::default();
    // Accumulate leaves in a vector to avoid recomputing across iterations.
    let leaves = generate_leaves!(*NUM_LEAVES.last().unwrap(), &mut rng);
    for num_leaves in NUM_LEAVES {
        // Benchmark the creation of a Merkle tree with the specified number of leaves.
        c.bench_function(&format!("MerkleTree/new/{num_leaves}"), |b| {
            b.iter(|| {
                let _tree = Testnet3::merkle_tree_bhp::<DEPTH>(&leaves[0..*num_leaves]).unwrap();
            })
        });
    }
}

fn append(c: &mut Criterion) {
    let mut rng = TestRng::default();
    // Accumulate all leaves in a vector to avoid recomputing across iterations.
    let leaves = generate_leaves!(*NUM_LEAVES.last().unwrap(), &mut rng);
    // Generate all of the leaves that will be appended to the tree.
    let new_leaves = generate_leaves!(*APPEND_SIZES.last().unwrap(), &mut rng);
    // For each number of leaves to append, benchmark the append operation.
    for num_leaves in NUM_LEAVES {
        for num_new_leaves in APPEND_SIZES {
            // Construct a Merkle tree with the specified number of leaves.
            let merkle_tree = Testnet3::merkle_tree_bhp::<DEPTH>(&leaves[..*num_leaves]).unwrap();
            c.bench_function(&format!("MerkleTree/append/{num_leaves}/{num_new_leaves}"), |b| {
                b.iter_batched(
                    || merkle_tree.clone(),
                    |mut merkle_tree| {
                        merkle_tree.append(&new_leaves[..*num_new_leaves]).unwrap();
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
}

fn update(c: &mut Criterion) {
    let mut rng = TestRng::default();
    // Accumulate leaves in a vector to avoid recomputing across iterations.
    let leaves = generate_leaves!(*NUM_LEAVES.last().unwrap(), &mut rng);
    // For each number of leaves to update, benchmark the update operation.
    for num_leaves in NUM_LEAVES {
        // Construct a vector of (index, new_leaf) pairs to update the tree with.
        // Note that we need to bound the number of updates since a large number of sequential singular updates to exceedingly long.
        let updates = generate_leaves!(std::cmp::min(*UPDATE_SIZES.last().unwrap(), 10_000), &mut rng)
            .into_iter()
            .map(|leaf| {
                let index: usize = Uniform::rand(&mut rng);
                (index % num_leaves, leaf)
            })
            .collect::<Vec<_>>();

        for num_new_leaves in UPDATE_SIZES {
            // Construct a Merkle tree with the specified number of leaves.
            let merkle_tree = Testnet3::merkle_tree_bhp::<DEPTH>(&leaves[..*num_leaves]).unwrap();

            c.bench_function(&format!("MerkleTree/update/{num_leaves}/{num_new_leaves}"), |b| {
                b.iter_batched(
                    || merkle_tree.clone(),
                    |mut merkle_tree| {
                        for (index, new_leaf) in updates.iter().take(*num_new_leaves) {
                            merkle_tree.update(*index, new_leaf).unwrap();
                        }
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
}

fn update_many(c: &mut Criterion) {
    let mut rng = TestRng::default();
    // Accumulate leaves in a vector to avoid recomputing across iterations.
    let leaves = generate_leaves!(*NUM_LEAVES.last().unwrap(), &mut rng);
    // For each number of leaves to update, benchmark the update operation.
    for num_leaves in NUM_LEAVES {
        // Generate all of the updates that will be applied to the tree.
        // Note that we generate 2 * MAX_UPDATE_SIZE leaves to increase the number of unique of updates.
        let mut updates = generate_leaves!(2 * *UPDATE_SIZES.last().unwrap(), &mut rng)
            .into_iter()
            .map(|leaf| {
                let index: usize = Uniform::rand(&mut rng);
                (index % num_leaves, leaf)
            })
            .collect::<Vec<_>>();
        updates.sort_by_key(|(a, _)| *a);
        updates.reverse();
        updates.dedup_by_key(|(a, _)| *a);

        for num_new_leaves in UPDATE_SIZES {
            // Construct a Merkle tree with the specified number of leaves.
            let merkle_tree = Testnet3::merkle_tree_bhp::<DEPTH>(&leaves[..*num_leaves]).unwrap();
            let num_new_leaves = std::cmp::min(*num_new_leaves, updates.len());
            c.bench_function(&format!("MerkleTree/update_many/{num_leaves}/{num_new_leaves}",), |b| {
                b.iter_batched(
                    || merkle_tree.clone(),
                    |mut merkle_tree| {
                        merkle_tree.update_many(&updates[..num_new_leaves]).unwrap();
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
}

fn update_vs_update_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("UpdateVSUpdateMany");
    let mut rng = TestRng::default();
    // Accumulate leaves in a vector to avoid recomputing across iterations.
    let max_leaves = 2usize.saturating_pow(MAX_INSTANTIATED_DEPTH as u32);
    let leaves = generate_leaves!(max_leaves, &mut rng);
    for depth in 1..=MAX_INSTANTIATED_DEPTH {
        // Compute the number of leaves at this depth.
        let num_leaves = 2usize.saturating_pow(depth as u32);
        // Construct a Merkle tree with the specified number of leaves.
        let tree = Testnet3::merkle_tree_bhp::<DEPTH>(&leaves[..num_leaves]).unwrap();
        // Generate a new leaf and select a random index to update.
        let index: usize = Uniform::rand(&mut rng);
        let index = index % num_leaves;
        let new_leaf = generate_leaves!(1, &mut rng).pop().unwrap();
        // Benchmark the standard update operation.
        group.bench_with_input(BenchmarkId::new("Single", &format!("{depth}")), &new_leaf, |b, new_leaf| {
            b.iter_batched(|| tree.clone(), |mut tree| tree.update(index, new_leaf), BatchSize::SmallInput)
        });
        // Benchmark the `update_many` operation.
        group.bench_with_input(
            BenchmarkId::new("Batch", &format!("{depth}")),
            &vec![(index, new_leaf)],
            |b, updates| b.iter_batched(|| tree.clone(), |mut tree| tree.update_many(updates), BatchSize::SmallInput),
        );
    }
}

criterion_group! {
    name = merkle_tree;
    config = Criterion::default().sample_size(10);
    targets = new, append, update, update_many, update_vs_update_many
}
criterion_main!(merkle_tree);
