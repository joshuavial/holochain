//! Mock host data for Kitsune to work with in tests. This is needed to create reasonably realistic tests that can exercise a range of Kitsune behaviour.
//!

use fixt::prelude::*;
use kitsune_p2p_bin_data::{KitsuneBinType, KitsuneOpHash, KitsuneOpData, KOp};
use kitsune_p2p_fetch::RoughSized;
use kitsune_p2p_timestamp::Timestamp;
use kitsune_p2p_types::{dht_arc::DhtLocation, KOpHash};
use kitsune_p2p_types::KSpace;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestHostOp {
    space: KSpace,
    hash: KitsuneOpHash,
    authored_at: Timestamp,
    size: u32,
}

impl TestHostOp {
    pub fn new(space: KSpace) -> Self {
        Self {
            space,
            hash: generated_hash(),
            authored_at: Timestamp::now(),
            size: fixt!(u8) as u32 + 1, // +1 because we don't want this to be 0
        }
    }

    // pub fn with_hash(mut self, hash: KitsuneOpHash) -> Self {
    //     self.hash = corrected_locaion_hash(hash);
    //     self
    // }

    pub fn space(&self) -> KSpace {
        self.space.clone()
    }

    pub fn kitsune_hash(&self) -> KitsuneOpHash {
        self.hash.clone()
    }

    pub fn hash(&self) -> [u8; 32] {
        // Assumes 32 byte hash, followed by 4 byte location
        self.hash[..32].try_into().unwrap()
    }

    pub fn location(&self) -> DhtLocation {
        self.hash.get_loc()
    }

    pub fn authored_at(&self) -> Timestamp {
        self.authored_at
    }

    pub fn size(&self) -> u32 {
        self.size
    }
}

impl Into<RoughSized<KOpHash>> for TestHostOp {
    fn into(self) -> RoughSized<KOpHash> {
        RoughSized::new(self.kitsune_hash().into(), Some(36.into()))
    }
}

impl Into<KOp> for TestHostOp {
    fn into(self) -> KOp {
        let str = serde_json::to_string(&self).unwrap();
        KitsuneOpData::new(str.into_bytes())
    }
}

impl From<KOp> for TestHostOp {
    fn from(op: KOp) -> Self {
        let str = String::from_utf8(op.0.clone()).unwrap();
        serde_json::from_str(&str).unwrap()
    }
}

fn generated_hash() -> KitsuneOpHash {
    let mut buf = vec![];
    buf.extend_from_slice(&fixt!(ThirtyTwoBytes)); // A random hash
    buf.extend(&dht_location(buf.as_slice()[..32].try_into().unwrap()));

    KitsuneOpHash::new(buf)
}

// fn corrected_location_hash(hash: KitsuneOpHash) -> KitsuneOpHash {
//     let mut buf = vec![];
//     buf.extend_from_slice(&hash.get_bytes()[..32]); // Keep the hash
//     buf.extend(&dht_location(buf.as_slice()[..32].try_into().unwrap()));

//     KitsuneOpHash::new(buf)
// }

// Ideally this would match the implementation in `holo_dht_location_bytes`
#[cfg(feature = "test_utils")]
pub fn dht_location(data: &[u8; 32]) -> [u8; 4] {
    let hash = blake2b_simd::Params::new()
        .hash_length(16)
        .hash(data)
        .as_bytes()
        .to_vec();

    let mut out = [hash[0], hash[1], hash[2], hash[3]];
    for i in (4..16).step_by(4) {
        out[0] ^= hash[i];
        out[1] ^= hash[i + 1];
        out[2] ^= hash[i + 2];
        out[3] ^= hash[i + 3];
    }
    out
}
