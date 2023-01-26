use std::collections::HashSet;

use libp2p::Swarm;
use log::info;

use crate::event::{App, BLOCK_TOPIC};

// 获取节点列表
pub fn get_list_peers(swarm: &Swarm<App>) -> Vec<String> {
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().map(|p| p.to_string()).collect()
}

// 处理打印节点命令
pub fn handle_print_peers(swarm: &Swarm<App>) {
    let peers = get_list_peers(swarm);
    info!("当前节点数:{} ",peers.len());
    peers.iter().for_each(|p| info!("[{}]", p));
}

// 处理打印区块命令
pub fn handle_print_chain(swarm: &Swarm<App>) {
    info!("本地块:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().chain.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}

// 处理创建区块命令
pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<App>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        // 最后一块开始挖矿
        let latest_block = behaviour
            .chain
            .blocks
            .last()
            .expect("there is at least one block");

        // 挖矿
        let block = latest_block.mine_block(data.to_owned());
        let json = serde_json::to_string(&block).expect("can jsonify request");

        // 添加到本地区块链
        behaviour.chain.blocks.push(block);
        info!("开始广播新块");
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}
