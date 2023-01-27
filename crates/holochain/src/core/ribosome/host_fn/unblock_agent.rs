use crate::core::ribosome::CallContext;
use crate::core::ribosome::RibosomeT;
use std::sync::Arc;
use holochain_wasmer_host::prelude::*;

pub fn unblock_agent(
    _ribosome: Arc<impl RibosomeT>,
    _call_context: Arc<CallContext>,
    _input: holochain_zome_types::block::UnblockAgentInput,
) -> Result<(), RuntimeError> {
    unreachable!();
}
