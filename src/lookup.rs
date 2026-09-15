use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::node::{Contact, NodeId, RoutingTable, K};
use crate::rpc::{NetworkManager, RpcPayload};

pub const ALPHA: usize = 3;
pub const RPC_TIMEOUT: Duration = Duration::from_secs(5);
pub const RPC_RETRIES: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateState {
    Unqueried,
    InFlight,
    Queried,
    Failed,
}

#[derive(Clone, Debug)]
struct Candidate {
    contact: Contact,
    distance: NodeId,
    state: CandidateState,
}

pub enum LookupValueResult {
    Found {
        value: Vec<u8>,
        from: Contact,
    },
    ClosestNodes(Vec<Contact>),
}

pub struct LookupEngine;

impl LookupEngine {
    pub async fn lookup_nodes(
        network: &Arc<NetworkManager>,
        routing_table: &Arc<RwLock<RoutingTable>>,
        target: NodeId,
    ) -> Vec<Contact> {
        let local_id = network.local_id;

        // Initialize shortlist with k closest known nodes
        let initial_contacts = {
            let rt = routing_table.read().await;
            rt.find_closest(&target, K)
        };

        let mut shortlist: Vec<Candidate> = initial_contacts
            .into_iter()
            .filter(|c| c.id != local_id)
            .map(|c| {
                let dist = c.id.distance(&target);
                Candidate {
                    contact: c,
                    distance: dist,
                    state: CandidateState::Unqueried,
                }
            })
            .collect();

        shortlist.sort_by(|a, b| a.distance.cmp(&b.distance));

        let mut hop = 0;
        loop {
            // Find up to ALPHA closest unqueried candidates
            let mut to_query = Vec::new();
            for cand in shortlist.iter_mut() {
                if cand.state == CandidateState::Unqueried {
                    cand.state = CandidateState::InFlight;
                    to_query.push(cand.contact.clone());
                    if to_query.len() >= ALPHA {
                        break;
                    }
                }
            }

            if to_query.is_empty() {
                // No more nodes to query
                break;
            }

            hop += 1;
            println!(
                "[LOOKUP] Hop {}: querying {} nodes in parallel for target {}",
                hop,
                to_query.len(),
                target
            );

            // Query in parallel
            let mut join_set = tokio::task::JoinSet::new();
            for contact in to_query {
                let net = Arc::clone(network);
                let tgt = target;
                join_set.spawn(async move {
                    let res = net
                        .call(
                            contact.addr,
                            RpcPayload::FindNode { target_id: tgt },
                            RPC_TIMEOUT,
                            RPC_RETRIES,
                        )
                        .await;
                    (contact, res)
                });
            }

            let mut new_contacts = Vec::new();
            while let Some(res) = join_set.join_next().await {
                if let Ok((contact, rpc_res)) = res {
                    match rpc_res {
                        Ok(reply) => {
                            // Mark as queried
                            if let Some(cand) = shortlist.iter_mut().find(|c| c.contact.id == contact.id) {
                                cand.state = CandidateState::Queried;
                            }
                            if let RpcPayload::FindNodeResp { contacts } = reply.payload {
                                for nc in contacts {
                                    if nc.id != local_id {
                                        new_contacts.push(nc);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("[LOOKUP] Node {} query failed: {}", contact.addr, e);
                            if let Some(cand) = shortlist.iter_mut().find(|c| c.contact.id == contact.id) {
                                cand.state = CandidateState::Failed;
                            }
                        }
                    }
                }
            }

            // Add newly discovered contacts to shortlist and routing table
            for nc in new_contacts {
                {
                    let mut rt = routing_table.write().await;
                    rt.update(nc.clone());
                }
                if !shortlist.iter().any(|c| c.contact.id == nc.id) {
                    let dist = nc.id.distance(&target);
                    shortlist.push(Candidate {
                        contact: nc,
                        distance: dist,
                        state: CandidateState::Unqueried,
                    });
                }
            }

            // Re-sort shortlist by distance to target
            shortlist.sort_by(|a, b| a.distance.cmp(&b.distance));

            // Stop condition: if top K candidates have all been queried or failed
            let top_k_queried = shortlist
                .iter()
                .take(K)
                .all(|c| c.state == CandidateState::Queried || c.state == CandidateState::Failed);
            if top_k_queried && shortlist.iter().take(K).any(|c| c.state == CandidateState::Queried) {
                break;
            }
        }

        let start_time = std::time::Instant::now();

        // Return up to K closest contacts that responded (or are known)
        let result: Vec<Contact> = shortlist
            .into_iter()
            .filter(|c| c.state != CandidateState::Failed)
            .take(K)
            .map(|c| c.contact)
            .collect();

        let latency_ms = start_time.elapsed().as_millis() as u64;
        network.metrics.record_lookup(hop, latency_ms);

        println!(
            "[LOOKUP] Iterative lookup for {} completed in {} hops ({:?}); returned {} nodes",
            target,
            hop,
            start_time.elapsed(),
            result.len()
        );

        result
    }

    pub async fn lookup_value(
        network: &Arc<NetworkManager>,
        routing_table: &Arc<RwLock<RoutingTable>>,
        key: NodeId,
    ) -> LookupValueResult {
        let local_id = network.local_id;

        let initial_contacts = {
            let rt = routing_table.read().await;
            rt.find_closest(&key, K)
        };

        let mut shortlist: Vec<Candidate> = initial_contacts
            .into_iter()
            .filter(|c| c.id != local_id)
            .map(|c| {
                let dist = c.id.distance(&key);
                Candidate {
                    contact: c,
                    distance: dist,
                    state: CandidateState::Unqueried,
                }
            })
            .collect();

        shortlist.sort_by(|a, b| a.distance.cmp(&b.distance));

        let mut hop = 0;
        loop {
            let mut to_query = Vec::new();
            for cand in shortlist.iter_mut() {
                if cand.state == CandidateState::Unqueried {
                    cand.state = CandidateState::InFlight;
                    to_query.push(cand.contact.clone());
                    if to_query.len() >= ALPHA {
                        break;
                    }
                }
            }

            if to_query.is_empty() {
                break;
            }

            hop += 1;
            println!(
                "[LOOKUP] Hop {}: querying {} nodes for key {}",
                hop,
                to_query.len(),
                key
            );

            let mut join_set = tokio::task::JoinSet::new();
            for contact in to_query {
                let net = Arc::clone(network);
                let k = key;
                join_set.spawn(async move {
                    let res = net
                        .call(
                            contact.addr,
                            RpcPayload::FindValue { key: k },
                            RPC_TIMEOUT,
                            RPC_RETRIES,
                        )
                        .await;
                    (contact, res)
                });
            }

            let mut new_contacts = Vec::new();
            while let Some(res) = join_set.join_next().await {
                if let Ok((contact, rpc_res)) = res {
                    match rpc_res {
                        Ok(reply) => {
                            if let Some(cand) = shortlist.iter_mut().find(|c| c.contact.id == contact.id) {
                                cand.state = CandidateState::Queried;
                            }
                            match reply.payload {
                                RpcPayload::FindValueResp {
                                    value: Some(val), ..
                                } => {
                                    println!(
                                        "[LOOKUP] Value for key {} found at node {} in {} hops!",
                                        key, contact.addr, hop
                                    );
                                    return LookupValueResult::Found {
                                        value: val,
                                        from: contact,
                                    };
                                }
                                RpcPayload::FindValueResp {
                                    value: None,
                                    contacts,
                                } => {
                                    for nc in contacts {
                                        if nc.id != local_id {
                                            new_contacts.push(nc);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            println!("[LOOKUP] Node {} failed during value lookup: {}", contact.addr, e);
                            if let Some(cand) = shortlist.iter_mut().find(|c| c.contact.id == contact.id) {
                                cand.state = CandidateState::Failed;
                            }
                        }
                    }
                }
            }

            for nc in new_contacts {
                {
                    let mut rt = routing_table.write().await;
                    rt.update(nc.clone());
                }
                if !shortlist.iter().any(|c| c.contact.id == nc.id) {
                    let dist = nc.id.distance(&key);
                    shortlist.push(Candidate {
                        contact: nc,
                        distance: dist,
                        state: CandidateState::Unqueried,
                    });
                }
            }

            shortlist.sort_by(|a, b| a.distance.cmp(&b.distance));

            let top_k_queried = shortlist
                .iter()
                .take(K)
                .all(|c| c.state == CandidateState::Queried || c.state == CandidateState::Failed);
            if top_k_queried && shortlist.iter().take(K).any(|c| c.state == CandidateState::Queried) {
                break;
            }
        }

        let result_nodes: Vec<Contact> = shortlist
            .into_iter()
            .filter(|c| c.state != CandidateState::Failed)
            .take(K)
            .map(|c| c.contact)
            .collect();

        println!(
            "[LOOKUP] Value lookup for key {} finished in {} hops (not found, returning {} closest nodes)",
            key,
            hop,
            result_nodes.len()
        );

        LookupValueResult::ClosestNodes(result_nodes)
    }
}
