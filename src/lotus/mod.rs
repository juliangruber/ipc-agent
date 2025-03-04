// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: MIT
///! The lotus api to interact with lotus node
use std::collections::HashMap;
use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use cid::Cid;
use fvm_shared::address::Address;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::econ::TokenAmount;
use ipc_gateway::{BottomUpCheckpoint, TopDownCheckpoint};
use ipc_sdk::cross::CrossMsg;
use ipc_sdk::subnet_id::SubnetID;
use serde::de::DeserializeOwned;

use crate::lotus::message::chain::GetTipSetByHeightResponse;
use message::chain::ChainHeadResponse;
use message::mpool::{MpoolPushMessage, MpoolPushMessageResponseInner};
use message::state::{ReadStateResponse, StateWaitMsgResponse};
use message::wallet::{WalletKeyType, WalletListResponse};

use crate::lotus::message::ipc::{IPCReadGatewayStateResponse, IPCReadSubnetActorStateResponse};
use crate::manager::SubnetInfo;

use self::message::CIDMap;

pub mod client;
mod json;
pub mod message;
#[cfg(test)]
mod tests;

/// The network version of lotus network.
/// see https://github.com/filecoin-project/go-state-types/blob/f6fd668a32b4b4a0bc39fd69d8a5f8fb11f49461/network/version.go#L7
pub type NetworkVersion = u32;

/// The Lotus client api to interact with the Lotus node.
#[async_trait]
pub trait LotusClient {
    /// Push the message to memory pool, see: https://lotus.filecoin.io/reference/lotus/mpool/#mpoolpushmessage
    async fn mpool_push_message(
        &self,
        msg: MpoolPushMessage,
    ) -> Result<MpoolPushMessageResponseInner>;

    /// Push the unsigned message to memory pool. This will ask the local key store to sign the message.
    /// In this case, make sure `from` is actually present in the local key store.
    /// See: https://lotus.filecoin.io/reference/lotus/mpool/#mpoolpush
    async fn mpool_push(&self, mut msg: MpoolPushMessage) -> Result<Cid>;

    /// Wait for the message cid of a particular nonce, see: https://lotus.filecoin.io/reference/lotus/state/#statewaitmsg
    async fn state_wait_msg(&self, cid: Cid) -> Result<StateWaitMsgResponse>;

    /// Returns the name of the network the node is synced to, see https://lotus.filecoin.io/reference/lotus/state/#statenetworkname
    async fn state_network_name(&self) -> Result<String>;

    /// Returns the network version at the given tipset, see https://lotus.filecoin.io/reference/lotus/state/#statenetworkversion
    async fn state_network_version(&self, tip_sets: Vec<Cid>) -> Result<NetworkVersion>;

    /// Returns the CID of the builtin actors manifest for the given network version, see https://github.com/filecoin-project/lotus/blob/master/documentation/en/api-v1-unstable-methods.md#stateactormanifestcid
    async fn state_actor_code_cids(
        &self,
        network_version: NetworkVersion,
    ) -> Result<HashMap<String, Cid>>;

    /// Get the default wallet of the node, see: https://lotus.filecoin.io/reference/lotus/wallet/#walletdefaultaddress
    async fn wallet_default(&self) -> Result<Address>;

    /// List the wallets in the node, see: https://lotus.filecoin.io/reference/lotus/wallet/#walletlist
    async fn wallet_list(&self) -> Result<WalletListResponse>;

    /// Create a new wallet, see: https://lotus.filecoin.io/reference/lotus/wallet/#walletnew
    async fn wallet_new(&self, key_type: WalletKeyType) -> Result<String>;

    /// Get the balance of an address
    async fn wallet_balance(&self, address: &Address) -> Result<TokenAmount>;

    /// Read the state of the address at tipset, see: https://lotus.filecoin.io/reference/lotus/state/#statereadstate
    async fn read_state<State: DeserializeOwned + Debug>(
        &self,
        address: Address,
        tipset: Cid,
    ) -> Result<ReadStateResponse<State>>;

    /// Returns the current head of the chain.
    /// See: https://lotus.filecoin.io/reference/lotus/chain/#chainhead
    async fn chain_head(&self) -> Result<ChainHeadResponse>;

    /// Returns the heaviest epoch for the chain
    async fn current_epoch(&self) -> Result<ChainEpoch>;

    /// GetTipsetByHeight from the underlying chain
    async fn get_tipset_by_height(
        &self,
        epoch: ChainEpoch,
        tip_set: Cid,
    ) -> Result<GetTipSetByHeightResponse>;

    async fn ipc_submit_top_down_checkpoint(
        &self,
        gateway_addr: Address,
        validator: &Address,
        checkpoint: TopDownCheckpoint,
    ) -> Result<ChainEpoch>;

    async fn ipc_get_prev_checkpoint_for_child(
        &self,
        gateway_addr: &Address,
        child_subnet_id: &SubnetID,
    ) -> Result<Option<CIDMap>>;

    /// Returns the checkpoint template at `epoch`.
    async fn ipc_get_checkpoint_template(
        &self,
        gateway_addr: &Address,
        epoch: ChainEpoch,
    ) -> Result<BottomUpCheckpoint>;

    /// Returns the checkpoint committed for an epoch in a child subnet.
    async fn ipc_get_checkpoint(
        &self,
        subnet_id: &SubnetID,
        epoch: ChainEpoch,
    ) -> Result<BottomUpCheckpoint>;

    /// Returns the state of the gateway actor at `tip_set`.
    async fn ipc_read_gateway_state(
        &self,
        gateway_addr: &Address,
        tip_set: Cid,
    ) -> Result<IPCReadGatewayStateResponse>;

    /// Returns the state of the subnet actor at `tip_set`.
    async fn ipc_read_subnet_actor_state(
        &self,
        subnet_id: &SubnetID,
        tip_set: Cid,
    ) -> Result<IPCReadSubnetActorStateResponse>;

    /// Returns the list of subnets in a gateway.
    async fn ipc_list_child_subnets(&self, gateway_addr: Address) -> Result<Vec<SubnetInfo>>;

    /// Determines if a validator has already voted for a bottomup checkpoint
    /// at certain epoch
    async fn ipc_validator_has_voted_bottomup(
        &self,
        subnet_id: &SubnetID,
        epoch: ChainEpoch,
        validator: &Address,
    ) -> Result<bool>;

    /// Determines if a validator has already voted for a topdown checkpoint
    /// at certain epoch
    async fn ipc_validator_has_voted_topdown(
        &self,
        gateway_addr: &Address,
        epoch: ChainEpoch,
        validator: &Address,
    ) -> Result<bool>;

    /// Returns the top-down messages committed for propagation from
    /// a specific `nonce` at a specific tipset
    async fn ipc_get_topdown_msgs(
        &self,
        subnet_id: &SubnetID,
        gateway_addr: &Address,
        tip_set: Cid,
        nonce: u64,
    ) -> Result<Vec<CrossMsg>>;

    /// Gets the genesis epoch at which a subnet was registered in the parent
    async fn ipc_get_genesis_epoch_for_subnet(
        &self,
        subnet_id: &SubnetID,
        gateway_addr: Address,
    ) -> Result<ChainEpoch>;

    /// Returns the list of checkpoints from a subnet actor for the given epoch range.
    async fn ipc_list_checkpoints(
        &self,
        subnet_id: SubnetID,
        from_epoch: ChainEpoch,
        to_epoch: ChainEpoch,
    ) -> Result<Vec<BottomUpCheckpoint>>;
}
