use log::{error, warn};

use crate::{block::Block, DIFFICULTY_PREFIX};

pub struct Chain {
    pub blocks: Vec<Block>,
}

impl Chain {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }

    pub fn genesis(&mut self) {
        let mut g = Block::genesis();
        g.hash = hex::encode(g.calculate_hash());
        self.blocks.push(g);
    }

    pub fn try_add_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if self.is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("非法块");
        }
    }

    pub fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("块 id: {} 前一个块hash不同", block.id);
            return false;
        } else if !hex::decode(&block.hash)
            .expect("can decode from hex")
            .starts_with(DIFFICULTY_PREFIX.as_bytes())
        {
            warn!("块id: {} 难度不合要求: {}", block.id, DIFFICULTY_PREFIX);
            return false;
        } else if block.id != previous_block.id + 1 {
            warn!("块id: {} 块id不匹配: {}", block.id, previous_block.id);
            return false;
        } else if hex::encode(previous_block.calculate_hash()) != block.previous_hash {
            warn!(
                "块id: {} 和前块id:{} hash不匹配 ",
                block.id, previous_block.id
            );
            return false;
        }
        true
    }

    pub fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first) {
                return false;
            }
        }
        true
    }

    // We always choose the longest valid chain
    pub fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);

        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if is_remote_valid && !is_local_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chains are both invalid");
        }
    }
}

#[test]
fn is_chain_valid_test() {
    let mut c = Chain::new();
    c.genesis();
    let b = c.blocks.last().expect("should not happen").mine_block(String::from("value"));
    c.try_add_block(b);
    assert_eq!(c.blocks.len(), 2);
    assert_eq!(c.is_chain_valid(c.blocks.as_slice()), true);
}
