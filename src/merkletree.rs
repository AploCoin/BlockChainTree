use sha2::Digest;
use sha2::Sha256;
use std::{convert::TryInto, sync::Arc};

static PADDING_HASH: [u8; 32] = *b"\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff";

#[derive(Debug)]
pub struct MerkleTree {
    array_representation: Arc<[[u8; 32]]>,
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

impl MerkleTree {
    pub fn build_tree(items: &[[u8; 32]]) -> MerkleTree {
        let closest_power_2 = find_closest_power_of_2(items.len());
        let depth = closest_power_2 + 1;
        let leaves_amount = 2usize.pow(closest_power_2 as u32);
        let nodes_total = (leaves_amount * 2) - 1;

        let mut array_representation = vec![PADDING_HASH; nodes_total];

        for (item, leaf_index) in items.iter().zip(nodes_total - leaves_amount..nodes_total) {
            unsafe {
                *(array_representation.get_unchecked_mut(leaf_index)) = *item;
            }
        }

        for index in nodes_total - (leaves_amount - items.len())..nodes_total {
            unsafe {
                *(array_representation.get_unchecked_mut(index)) = PADDING_HASH;
            }
        }

        for left_index in (1..nodes_total - 1).step_by(2).rev() {
            //let mut hasher = Sha256::new();
            let mut to_hash = [0u8; 32];
            unsafe {
                for (index, (left, right)) in array_representation
                    .get_unchecked(left_index)
                    .iter()
                    .zip(array_representation.get_unchecked(left_index + 1).iter())
                    .enumerate()
                {
                    *to_hash.get_unchecked_mut(index) = *left & *right;
                }
                //hasher.reset();

                let hash = Sha256::digest(&to_hash);

                *(array_representation.get_unchecked_mut((left_index - 1) / 2)) =
                    hash.as_slice().try_into().unwrap_unchecked();
            }
        }

        MerkleTree {
            array_representation: Arc::<[[u8; 32]]>::from(array_representation),
            depth,
            initial_amount_of_inputs: items.len(),
        }
    }

    pub fn get_proof<'a>(&'a self, hash: &[u8; 32]) -> Vec<&'a [u8; 32]> {
        let mut to_return = vec![&[0u8; 32]; self.depth];

        let mut to_return_index: usize = 0;

        let leaves_amount = 2usize.pow((self.depth - 1) as u32);

        let mut index: isize = 0;
        for rel_index in self.array_representation.len() - leaves_amount
            ..self.array_representation.len() - (leaves_amount - self.initial_amount_of_inputs)
        {
            index = rel_index as isize;
            unsafe {
                if self.array_representation.get_unchecked(rel_index) == hash {
                    break;
                }
            }
        }

        while index > 0 {
            let lsb_set = index & 1;
            let lsb_not_set = lsb_set ^ 1;

            // let mut sibling_index = index;

            index += lsb_set;
            index -= lsb_not_set;

            unsafe {
                *to_return.get_unchecked_mut(to_return_index) =
                    self.array_representation.get_unchecked(index as usize);
            }

            to_return_index += 1;
            index = (index - 1) / 2;
        }

        unsafe {
            *to_return.get_unchecked_mut(to_return_index) =
                self.array_representation.get_unchecked(0);
        }

        to_return
    }

    pub fn verify_proof(hash: &[u8; 32], root: &[u8; 32], proof: &[&[u8; 32]]) -> bool {
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
                *item &= *unsafe { idx.get_unchecked(n) };
            }
            hasher.update(calculated_root);
            calculated_root = unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() };
        }

        for i in 0..32 {
            unsafe {
                if root.get_unchecked(i) != calculated_root.get_unchecked(i) {
                    return false;
                }
            }
        }
        true
    }

    pub fn get_root(&self) -> &[u8; 32] {
        unsafe { self.array_representation.get_unchecked(0) }
    }

    // pub fn new() -> MerkleTree {
    //     todo!()
    // }

    // pub fn add_objects(&mut self, input: &Vec<[u8; 32]>) -> bool {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use super::MerkleTree;
    use rand::Rng;
    use std::time::Instant;

    #[test]
    fn merkle_tree_test() {
        let mut rng = rand::thread_rng();
        let data: &Vec<[u8; 32]> = &vec![[rng.gen(); 32]; 10000];
        let start = Instant::now();
        let tree = MerkleTree::build_tree(data);
        let duration = start.elapsed();

        println!("Time for building: {:?}", duration);

        let proof = tree.get_proof(&[1u8; 32]);

        let root = tree.get_root();

        let valid = MerkleTree::verify_proof(&[1u8; 32], root, &proof[0..proof.len() - 1]);

        println!("Root: {:?}", root);

        println!("Proof valid: {:?}", valid);

        println!("Proof: {:?}", proof);
    }
}
