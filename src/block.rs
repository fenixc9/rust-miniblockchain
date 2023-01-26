use chrono::Utc;
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
const DIFFICULTY_PREFIX: &str = "00";
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}

impl Block {
    pub fn genesis() -> Self {
        let mut b = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("创世区块"),
            data: String::from("创世区块"),
            nonce: 123,
            hash: String::default(),
        };
        b.hash = hex::encode(b.calculate_hash());
        b
    }

    pub fn calculate_hash(&self) -> Vec<u8> {
        let mut s = Sha256::new();
        s.update(self.id.to_le_bytes());
        s.update(self.timestamp.to_le_bytes());
        s.update(self.previous_hash.as_bytes());
        s.update(self.data.as_bytes());
        s.update(self.nonce.to_le_bytes());
        s.finalize().as_slice().to_owned()
    }

    pub fn mine_block(&self, data: String) -> Self {
        info!("开始挖矿");
        let mut newb = Block {
            id: self.id + 1,
            hash: String::default(),
            previous_hash: self.hash.clone(),
            timestamp: Utc::now().timestamp(),
            data,
            nonce: 0,
        };
        loop {
            let hash = newb.calculate_hash();
            if hash.starts_with(DIFFICULTY_PREFIX.as_bytes()) {
                info!("挖矿成功!  hash: {}", hex::encode(&hash),);
                newb.hash = hex::encode(hash);
                break;
            }
            newb.nonce += 1;
        }
        newb
    }
}


#[test]
fn t() {
    let mut b = Block::genesis();
    b.hash = hex::encode(b.calculate_hash());
    println!("{:?}", b);
}

#[test]
fn mine() {
    let mut b = Block::genesis();
    b.hash = hex::encode(b.calculate_hash());
    println!("{:?}", serde_json::json!(b));
    let s = b.mine_block(String::from("second block"));
    println!("{:?}", serde_json::json!(s));
}