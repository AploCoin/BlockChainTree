use sha2::{Sha256, Digest};
use std::convert::TryInto;


static PADDING_HASH:[u8;32] = *b"\xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff\
                                \xff\xff\xff\xff\xff\xff\xff\xff";

#[derive(Debug)]
pub struct MerkleTree{
    array_representation:Vec<Option<[u8;32]>>,
    depth:usize,
    initial_amount_of_inputs:usize
}


pub fn find_closest_power_of_2(number:usize) -> usize{
    let mut power:usize = 0;
    let mut x:usize = 1;
    while x<=number{
        power += 1;
        x = x << 1;
    }
    return power;
}

impl MerkleTree{
    pub fn new()->MerkleTree{
        return MerkleTree{
            array_representation:Vec::with_capacity(0),
            depth:0,
            initial_amount_of_inputs:0};
    }

    fn calculate_parents_hash(&self,index:usize)->Option<[u8;32]>{
        let left_child = (index*2)+1;
        if left_child >= self.array_representation.len(){
            return None;
        }
        let right_child = (index*2)+2;
        if right_child >= self.array_representation.len(){
            return None;
        }
        let mut hasher = Sha256::new();

        let mut hash_input:[u8;32];
        if self.array_representation[left_child] != None{
            hash_input = self.array_representation[left_child].unwrap();
        }else{
            hash_input = PADDING_HASH;
        }
        if self.array_representation[right_child] != None{
            for i in 0..32{
                hash_input[i] &= self.array_representation[right_child].unwrap()[i];
            }
        }

        hasher.update(hash_input);
        let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
        return Some(result);
    }

    fn populate_tree(&mut self,right_node:usize,right_branch:bool){
        if right_node <= 1{
            return
        }

        let parent_index:usize = (right_node-2)/2;

        self.array_representation[parent_index] = self.calculate_parents_hash(parent_index);
        self.populate_tree(right_node-2, !right_branch);

        if right_branch{
            self.populate_tree(parent_index, true);
        }
    }

    pub fn add_objects(&mut self,mut input:Vec<&[u8;32]>) -> bool{
        if self.array_representation.len() != 0{
            return false;
        }

        self.initial_amount_of_inputs = input.len();

        let initial_length = input.len();
        self.depth = find_closest_power_of_2(initial_length);
        if initial_length%2 != 0{
            for _ in initial_length..usize::pow(2,self.depth as u32){
                input.push(&PADDING_HASH);
            }
            self.depth = find_closest_power_of_2(input.len());
        }

        let amount_of_nodes:usize = usize::pow(2,self.depth as u32) - 1;

        self.array_representation.reserve(amount_of_nodes);

        for _ in 0..amount_of_nodes-input.len(){
            self.array_representation.push(None);
        }

        for inp in input.iter(){
            self.array_representation.push(Some(**inp));
        }
        self.populate_tree(self.array_representation.len(), true);

        return true;
    }

    pub fn check_node(&self, index:usize) -> bool{
        let parent_hash = self.calculate_parents_hash(index).unwrap();

        for i in 0..32{
            if self.array_representation[index].unwrap()[i] != parent_hash[i]{
                return false;
            }
        }

        return true
    }

    fn exists(&self, hash:&[u8;32]) -> Option<usize>{
        for i in self.array_representation.len()-self.initial_amount_of_inputs-1..self.array_representation.len(){
            if self.array_representation[i].is_none(){
                return None;
            }
            let mut equal:bool = true;
            for (first, second) in self.array_representation[i].unwrap().iter().zip(hash.iter()){
                if *first != *second{
                    equal = false;
                    break;
                }
            }
            if equal{
                return Some(i);
            }
        }
        return None;
    }

    pub fn get_proof<'a>(&'a self, hash:&[u8;32]) -> Result<Vec<&'a [u8;32]>,&'static str>{

        let starting_node_res = self.exists(hash);
        if starting_node_res.is_none(){
            return Err("No such hash found");
        }

        let mut starting_node:usize = starting_node_res.unwrap();

        let mut to_return:Vec<&'a [u8;32]> = Vec::with_capacity(self.depth);
        while starting_node != 0{
            if starting_node%2 == 0{
                match self.array_representation[starting_node-1]{
                    Some(ref data) => {to_return.push(&data);}
                    _ => {to_return.push(&PADDING_HASH);}
                }
                starting_node = (starting_node-2)/2;
            }else{
                match self.array_representation[starting_node+1]{
                    Some(ref data) => {to_return.push(&data);}
                    _ => {to_return.push(&PADDING_HASH);}
                }
                starting_node = (starting_node-1)/2;
            }
        }
        return Ok(to_return);
    }
    pub fn get_root<'a>(&'a self) -> &'a [u8;32]{
        return self.array_representation[0].as_ref().unwrap();
    }
}

pub fn verify_proof(hash:&[u8;32],
                    root:&[u8;32],
                    proof:Vec<&[u8;32]>)->bool{
    let mut hasher = Sha256::new();
    let mut calculated_root:[u8;32] = [0;32];

    for i in 0..32{
        calculated_root[i] = hash[i]&proof[0][i];
    }
    hasher.update(calculated_root);
    calculated_root = hasher.finalize().as_slice().try_into().unwrap();

    for i in 1..proof.len(){
        let mut hasher = Sha256::new();
        for n in 0..32{
            calculated_root[n] = calculated_root[n]&proof[i][n];
        }
        hasher.update(calculated_root);
        calculated_root = hasher.finalize().as_slice().try_into().unwrap();
    }

    for i in 0..32{
        if root[i] != calculated_root[i]{
            return false;
        }
    }
    return true;
}