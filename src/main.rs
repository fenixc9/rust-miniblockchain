use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info, debug};

use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};

const DIFFICULTY_PREFIX: &str = "00";
mod block;
mod chain;
mod event;
mod cmd;
use block::Block;
use chain::Chain;

#[tokio::main]
async fn main() {
    // 日志
    pretty_env_logger::init();

    info!("当前难度:{}", DIFFICULTY_PREFIX);
    info!("Peer Id: {}", event::PEER_ID.clone());
    let (chain_sender, mut chain_receiver) = mpsc::unbounded_channel();
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel();

    // p2p密钥
    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&event::KEYS)
        .expect("can create auth keys");

    // p2p配置
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // 创建应用
    let app = event::App::new(Chain::new(), chain_sender, init_sender.clone()).await;

    // 创建节点
    let mut swarm = SwarmBuilder::new(transp, app, *event::PEER_ID)
        .executor(Box::new(|fut| {
            spawn(fut);
        }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    // 节点开始监听
    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    // 延迟1秒，等待节点启动
    spawn(async move {
        sleep(Duration::from_secs(1)).await;
        init_sender.send(true).expect("can send init event");
    });

    // 主事件循环
    loop {
        let evt = {
            select! {
                // 从标准输入读取命令
                line = stdin.next_line() => Some(event::Event::Cmd(
                    line.expect("can get line").expect("can read line from stdin")
                )),

                response = chain_receiver.recv() => {
                    Some(event::Event::ChainResponse(response.expect("response exists")))
                },
                // 初始化
                _init = init_rcv.recv() => {
                    Some(event::Event::Init)
                }
                // 这里可以打印下swarm的消息
                _event = swarm.select_next_some() => {
                    debug!("swarm event: {:?}", _event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                event::Event::Init => {
                    // 创世区块
                    swarm.behaviour_mut().chain.genesis();

                    // 获取所有节点
                    let peers = cmd::get_list_peers(&swarm);

                    // 如果有节点，向最后一个节点请求区块链
                    if !peers.is_empty() {
                        let req = event::ChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        // floodsub广播
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(event::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                event::Event::ChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    // 发送给其他节点
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(event::CHAIN_TOPIC.clone(), json.as_bytes());
                }
                // 解析标准输入命令
                event::Event::Cmd(line) => match line.as_str() {
                    "ls p" => cmd::handle_print_peers(&swarm),
                    cmd if cmd.starts_with("ls c") => cmd::handle_print_chain(&swarm),
                    cmd if cmd.starts_with("create b") => {
                        cmd::handle_create_block(cmd, &mut swarm)
                    }
                    _ => error!("unknown command"),
                },
            }
        }
    }
}
