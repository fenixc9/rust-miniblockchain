use super::{Block, Chain};
use libp2p::{
    floodsub::{Floodsub, FloodsubEvent, Topic},
    identity,
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub blocks: Vec<Block>,
    pub receiver: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainRequest {
    pub from_peer_id: String,
}

pub enum Event {
    ChainResponse(ChainResponse),
    Cmd(String),
    Init,
}

#[derive(NetworkBehaviour)]
pub struct App {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub chain_sender: UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub chain: Chain,
}

impl App {
    pub async fn new(
        app: Chain,
        chain_sender: UnboundedSender<ChainResponse>,
        init_sender: UnboundedSender<bool>,
    ) -> Self {
        let mut app = Self {
            chain: app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default())
                .await
                .expect("can create mdns"),
            chain_sender,
            init_sender,
        };
        app.floodsub.subscribe(CHAIN_TOPIC.clone());
        app.floodsub.subscribe(BLOCK_TOPIC.clone());
        app
    }
}

// 处理FloodSub事件
impl NetworkBehaviourEventProcess<FloodsubEvent> for App {
    fn inject_event(&mut self, event: FloodsubEvent) {
        if let FloodsubEvent::Message(msg) = event {
            // 如果这里是别的节点返回的链，我们需对比下，选择最长的链
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data) {
                // clone 区块返回
                if resp.receiver == PEER_ID.to_string() {
                    info!("遍历区块:");
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));

                    // 校验并选择最长链
                    self.chain.blocks = self
                        .chain
                        .choose_chain(self.chain.blocks.clone(), resp.blocks);
                }
                // 如果这是别的节点查询我们的链，我们需要clone链返回
            } else if let Ok(resp) = serde_json::from_slice::<ChainRequest>(&msg.data) {
                let peer_id = resp.from_peer_id;
                // 询问本节点的区块
                if PEER_ID.to_string() == peer_id {
                    // clone区块发送给查询者，这里没有同步发送，而是交给channel异步处理
                    if let Err(e) = self.chain_sender.send(ChainResponse {
                        blocks: self.chain.blocks.clone(),
                        receiver: msg.source.to_string(),
                    }) {
                        error!("error sending response via channel, {}", e);
                    }
                }
                // 如果是别的节点发来的区块，我们尝试将其加入到区块链中
            } else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                info!("接受到新块 {}", msg.source.to_string());
                self.chain.try_add_block(block);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for App {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            // 发现新节点
            MdnsEvent::Discovered(discovered_list) => {
                // info!("发现新节点:{}", discovered_list.len());
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            // 节点过期
            MdnsEvent::Expired(expired_list) => {
                // info!("节点超时:{}", expired_list.len());
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
