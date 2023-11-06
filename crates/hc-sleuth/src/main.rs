use std::{path::PathBuf, str::FromStr};

use hc_sleuth::{report, Step};
use holochain_types::prelude::*;
use structopt::StructOpt;

fn main() {
    let opt = HcSleuth::from_args();

    match opt {
        HcSleuth::ShowGraph => {
            unimplemented!("showing graph not yet implemented");
            report(
                Step::Integrated {
                    by: "".into(),
                    op: DhtOpHash::from_raw_32(vec![0; 32]),
                },
                &Default::default(),
            );
        }
        HcSleuth::Query {
            op_hash,
            node,
            log_paths,
        } => {
            dbg!(log_paths);
            unimplemented!("command-line query not yet implemented")
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "hc-sleuth",
    about = "Examine the causal relationships between events in Holochain"
)]
pub enum HcSleuth {
    ShowGraph,
    Query {
        #[structopt(
            short = "h",
            long,
            help = "The action or entry hash to check for integration"
        )]
        op_hash: TargetHash,
        #[structopt(
            short,
            long,
            help = "The node ID which integrated (check the `tracing_scope` setting of your conductor config for this value)"
        )]
        node: String,
        log_paths: Vec<PathBuf>,
    },
}

#[derive(Debug, derive_more::Deref)]
pub struct TargetHash(DhtOpHash);

impl FromStr for TargetHash {
    type Err = HoloHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = DhtOpHashB64::from_b64_str(s)?;
        Ok(Self(hash.into()))
    }
}
