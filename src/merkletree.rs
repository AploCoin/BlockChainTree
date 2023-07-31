use error_stack::{Report, Result};
use sha2::{Digest, Sha256};
use std::convert::TryInto;

use crate::errors::*;

static PADDING_HASH: [u8; 32] = *b"\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff";

#[derive(Debug)]
pub struct MerkleTree {
    array_representation: Vec<Option<[u8; 32]>>,
    depth: usize,
    initial_amount_of_inputs: usize,
}

pub fn find_closest_power_of_2(number: usize) -> usize {
    let mut power: usize = 0;
    let mut x: usize = 1;
    while x <= number {
        power += 1;
        x <<= 1;
    }
    power
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleTree {
    pub fn new() -> MerkleTree {
        MerkleTree {
            array_representation: Vec::with_capacity(0),
            depth: 0,
            initial_amount_of_inputs: 0,
        }
    }

    fn calculate_parents_hash(&self, index: usize) -> Option<[u8; 32]> {
        let left_child = (index * 2) + 1;
        if left_child >= self.array_representation.len() {
            return None;
        }
        let right_child = (index * 2) + 2;
        if right_child >= self.array_representation.len() {
            return None;
        }
        let mut hasher = Sha256::new();

        let mut hash_input: [u8; 32];
        if self.array_representation[left_child].is_some() {
            hash_input = self.array_representation[left_child].unwrap();
        } else {
            hash_input = PADDING_HASH;
        }
        if self.array_representation[right_child].is_some() {
            for (idx, hinput) in hash_input.iter_mut().enumerate() {
                *hinput &= self.array_representation[right_child].unwrap()[idx];
            }
        }

        hasher.update(hash_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
        Some(result)
    }

    fn populate_tree(&mut self, right_node: usize, right_branch: bool) {
        if right_node <= 1 {
            return;
        }

        let parent_index: usize = (right_node - 2) / 2;

        self.array_representation[parent_index] = self.calculate_parents_hash(parent_index);
        self.populate_tree(right_node - 2, !right_branch);

        if right_branch {
            self.populate_tree(parent_index, true);
        }
    }

    pub fn add_objects(&mut self, input: &Vec<[u8; 32]>) -> bool {
        if !self.array_representation.is_empty() {
            return false;
        }

        self.initial_amount_of_inputs = input.len();

        let new_size = if self.initial_amount_of_inputs % 2 != 0 {
            let size = usize::pow(
                2,
                find_closest_power_of_2(self.initial_amount_of_inputs) as u32,
            );
            self.depth = find_closest_power_of_2(size);
            size
        } else {
            self.depth = find_closest_power_of_2(self.initial_amount_of_inputs);
            self.initial_amount_of_inputs
        };

        let amount_of_nodes: usize = usize::pow(2, self.depth as u32) - 1;

        self.array_representation.reserve(amount_of_nodes);

        for _ in 0..amount_of_nodes - new_size {
            self.array_representation.push(None);
        }

        for inp in input.iter() {
            self.array_representation.push(Some(*inp));
        }
        for _ in input.len()..new_size {
            self.array_representation.push(Some(PADDING_HASH));
        }
        self.populate_tree(self.array_representation.len(), true);

        true
    }

    pub fn check_node(&self, index: usize) -> bool {
        let parent_hash = self.calculate_parents_hash(index).unwrap();

        for (idx, phash) in parent_hash.iter().enumerate() {
            if self.array_representation[index].unwrap()[idx] != *phash {
                return false;
            }
        }

        true
    }

    fn exists(&self, hash: &[u8; 32]) -> Option<usize> {
        for i in self.array_representation.len() - self.initial_amount_of_inputs - 1
            ..self.array_representation.len()
        {
            self.array_representation[i]?;

            let mut equal: bool = true;
            for (first, second) in self.array_representation[i]
                .unwrap()
                .iter()
                .zip(hash.iter())
            {
                if *first != *second {
                    equal = false;
                    break;
                }
            }
            if equal {
                return Some(i);
            }
        }
        None
    }

    pub fn get_proof<'a>(&'a self, hash: &[u8; 32]) -> Result<Vec<&'a [u8; 32]>, MerkleTreeError> {
        let starting_node_res = self.exists(hash);
        if starting_node_res.is_none() {
            return Err(Report::new(MerkleTreeError::TreeError(
                MerkleTreeErrorKind::GettingProof,
            ))
            .attach_printable(format!(
                "hash: {:?} // {} doesn't exist",
                hash,
                std::str::from_utf8(hash).unwrap()
            )));
        }

        let mut starting_node: usize = starting_node_res.unwrap();

        let mut to_return: Vec<&'a [u8; 32]> = Vec::with_capacity(self.depth);
        while starting_node != 0 {
            if starting_node % 2 == 0 {
                match self.array_representation[starting_node - 1] {
                    Some(ref data) => {
                        to_return.push(data);
                    }
                    _ => {
                        to_return.push(&PADDING_HASH);
                    }
                }
                starting_node = (starting_node - 2) / 2;
            } else {
                match self.array_representation[starting_node + 1] {
                    Some(ref data) => {
                        to_return.push(data);
                    }
                    _ => {
                        to_return.push(&PADDING_HASH);
                    }
                }
                starting_node = (starting_node - 1) / 2;
            }
        }
        Ok(to_return)
    }
    pub fn get_root(&self) -> &[u8; 32] {
        return self.array_representation[0].as_ref().unwrap();
    }
}

pub fn verify_proof(hash: &[u8; 32], root: &[u8; 32], proof: Vec<&[u8; 32]>) -> bool {
    let mut hasher = Sha256::new();
    let mut calculated_root: [u8; 32] = [0; 32];

    for (n, i) in calculated_root.iter_mut().enumerate() {
        *i = hash[n] & proof[0][n];
    }
    hasher.update(calculated_root);
    calculated_root = unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() };

    for idx in proof.iter().skip(1) {
        let mut hasher = Sha256::new();

        for (n, item) in calculated_root.iter_mut().enumerate() {
            *item &= idx[n]
        }
        hasher.update(calculated_root);
        calculated_root = unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() };
    }

    for i in 0..32 {
        if root[i] != calculated_root[i] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::MerkleTree;

    #[test]
    fn merkle_tree_test() {
        let mut merkle_tree = MerkleTree::new();

        merkle_tree.add_objects(&vec![[1u8; 32], [1u8; 32], [1u8; 32], [1u8; 32], [1u8; 32]]);

        let root = *merkle_tree.get_root();
        println!("Root: {:?}", root);

        assert_eq!(
            [
                111, 57, 169, 50, 108, 105, 72, 100, 55, 246, 58, 248, 58, 198, 79, 115, 83, 127,
                186, 9, 169, 151, 207, 78, 205, 188, 34, 250, 175, 218, 23, 155
            ],
            root
        );
    }
}
