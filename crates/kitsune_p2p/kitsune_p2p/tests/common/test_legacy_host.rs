use futures::{channel::mpsc::Receiver, FutureExt, StreamExt};
use itertools::Itertools;
use kitsune_p2p::event::{FetchOpDataEvtQuery, KitsuneP2pEvent};
use kitsune_p2p_bin_data::{KOp, KitsuneAgent, KitsuneOpData, KitsuneSignature, KitsuneOpHash};
use kitsune_p2p_timestamp::Timestamp;
use kitsune_p2p_types::{
    agent_info::AgentInfoSigned,
    dht::{
        arq::LocalStorageConfig,
        spacetime::{Dimension, Topology},
        ArqStrat, PeerStrat,
    }, dht_arc::{DhtArcSet, DhtArc, DhtArcRange},
};
use std::{borrow::Borrow, collections::HashSet, sync::Arc, intrinsics::unreachable};

use super::{test_keystore, TestHostOp};

pub struct TestLegacyHost {
    _handle: tokio::task::JoinHandle<()>,
    keystore: Arc<
        futures::lock::Mutex<
            kitsune_p2p_types::dependencies::lair_keystore_api::prelude::LairClient,
        >,
    >,
}

impl TestLegacyHost {
    pub async fn start(
        agent_store: Arc<parking_lot::RwLock<Vec<AgentInfoSigned>>>,
        op_store: Arc<parking_lot::RwLock<Vec<TestHostOp>>>,
        receivers: Vec<Receiver<KitsuneP2pEvent>>,
    ) -> Self {
        let keystore = test_keystore();

        let handle = tokio::task::spawn({
            let keystore = keystore.clone();
            async move {
                let mut receiver = futures::stream::select_all(receivers).fuse();
                while let Some(evt) = receiver.next().await {
                    match evt {
                        KitsuneP2pEvent::PutAgentInfoSigned { respond, input, .. } => {
                            let mut store = agent_store.write();
                            let incoming_agents: HashSet<_> =
                                input.peer_data.iter().map(|p| p.agent.clone()).collect();
                            store.retain(|p: &AgentInfoSigned| !incoming_agents.contains(&p.agent));
                            store.extend(input.peer_data);
                            respond.respond(Ok(async move { Ok(()) }.boxed().into()))
                        }
                        KitsuneP2pEvent::QueryAgents { respond, input, .. } => {
                            let store = agent_store.read();
                            let agents = store
                                .iter()
                                .filter(|p| p.space == input.space)
                                .cloned()
                                .collect::<Vec<_>>();
                            respond.respond(Ok(async move { Ok(agents) }.boxed().into()))
                        }
                        KitsuneP2pEvent::QueryPeerDensity {
                            respond,
                            space,
                            dht_arc,
                            ..
                        } => {
                            let cutoff = std::time::Duration::from_secs(60 * 15);
                            let topology = Topology {
                                space: Dimension::standard_space(),
                                time: Dimension::time(std::time::Duration::from_secs(60 * 5)),
                                time_origin: Timestamp::now(),
                                time_cutoff: cutoff,
                            };
                            let now = Timestamp::now().as_millis() as u64;
                            let arcs = agent_store
                                .read()
                                .iter()
                                .filter_map(|agent: &AgentInfoSigned| {
                                    if agent.space == space && now < agent.expires_at_ms {
                                        Some(agent.storage_arc.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>();

                            let strat = PeerStrat::Quantized(ArqStrat::standard(
                                LocalStorageConfig::default(),
                            ));
                            let view = strat.view(topology, dht_arc, &arcs);

                            respond.respond(Ok(async move { Ok(view) }.boxed().into()))
                        }
                        KitsuneP2pEvent::Call {
                            respond, payload, ..
                        } => {
                            // Echo the request payload
                            respond.respond(Ok(async move { Ok(payload) }.boxed().into()))
                        }
                        KitsuneP2pEvent::QueryOpHashes { respond, input, .. } => {
                            // TODO nothing to send yet

                            let op_store = op_store.read();
                            let selected_ops: Vec<TestHostOp> = op_store.iter().filter(|op| {
                                if op.space() != input.space {
                                    return false;
                                }

                                if op.authored_at() < input.window.start && op.authored_at() > input.window.end {
                                    return false;
                                }

                                let intervals = input.arc_set.intervals();
                                if let Some(DhtArcRange::Full) = intervals.first() {

                                } else {
                                    let mut in_any = false;
                                    for interval in intervals {
                                        match interval {
                                            DhtArcRange::Bounded(lower, upper) => {
                                                if lower < op.location() && op.location() < upper {
                                                    in_any = true;
                                                    break;
                                                }
                                            }
                                            _ => unreachable!("Invalid input to host query for op hashes")
                                        }
                                    }

                                    if !in_any {
                                        return false;
                                    }
                                }

                                true
                            }).take(input.max_ops).sorted_by_key(|op| op.authored_at()).cloned().collect();

                            if selected_ops.len() > 0 {
                                let low_time = selected_ops.first().unwrap().authored_at();
                                let high_time = selected_ops.last().unwrap().authored_at();

                                respond.respond(Ok(async move { Ok(Some(selected_ops.into_iter().map(|op| op.kitsune_hash()))) }.boxed().into()))
                            }
                            
                        }
                        KitsuneP2pEvent::FetchOpData { respond, input, .. } => {
                            let result = match input.query {
                                FetchOpDataEvtQuery::Hashes { op_hash_list, .. } => {
                                    let search_hashes =
                                        op_hash_list.into_iter().collect::<HashSet<_>>();
                                        let op_store = op_store.read();
                                    let matched_host_data = op_store.iter().filter(|op| {
                                        op.space() == input.space
                                            && search_hashes.contains(&op.kitsune_hash())
                                    });

                                    matched_host_data
                                        .map(|h| {
                                            (
                                                Arc::new(h.kitsune_hash()),
                                                KitsuneOpData::new(
                                                    std::iter::repeat(0)
                                                        .take(h.size() as usize)
                                                        .collect(),
                                                ),
                                            )
                                        })
                                        .collect()
                                }
                                _ => {
                                    unimplemented!("Only know how to handle Hashes variant");
                                }
                            };

                            respond.respond(Ok(async move { Ok(result) }.boxed().into()))
                        }
                        KitsuneP2pEvent::SignNetworkData { respond, input, .. } => {
                            let mut key = [0; 32];
                            key.copy_from_slice(&input.agent.0.as_slice());
                            let sig = keystore
                                .lock()
                                .await
                                .sign_by_pub_key(
                                    key.into(),
                                    None,
                                    input.data.as_slice().to_vec().into(),
                                )
                                .await
                                .unwrap();
                            respond.respond(Ok(async move { Ok(KitsuneSignature(sig.0.to_vec())) }
                                .boxed()
                                .into()))
                        }
                        _ => todo!("Unhandled event {:?}", evt),
                    }
                }
            }
        });

        Self {
            _handle: handle,
            keystore,
        }
    }

    pub async fn create_agent(&self) -> KitsuneAgent {
        let ks = self.keystore.lock().await;
        let tag = nanoid::nanoid!();
        let info = ks.new_seed(tag.into(), None, false).await.unwrap();
        KitsuneAgent(info.ed25519_pub_key.0.to_vec())
    }
}
