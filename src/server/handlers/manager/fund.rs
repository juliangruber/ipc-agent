// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: MIT
//! Fund operation in the gateway actor

use crate::server::handlers::manager::subnet::SubnetManagerPool;
use crate::server::{check_subnet, handlers, parse_from, JsonRPCRequestHandler};
use anyhow::anyhow;
use async_trait::async_trait;
use fvm_shared::address::Address;
use fvm_shared::clock::ChainEpoch;
use ipc_sdk::subnet_id::SubnetID;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct FundParams {
    pub subnet: String,
    pub from: Option<String>,
    pub to: Option<String>,
    /// In whole FIL
    pub amount: f64,
}

/// The fund json rpc method handler.
pub(crate) struct FundHandler {
    pool: Arc<SubnetManagerPool>,
}

impl FundHandler {
    pub(crate) fn new(pool: Arc<SubnetManagerPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JsonRPCRequestHandler for FundHandler {
    type Request = FundParams;
    type Response = ChainEpoch;

    async fn handle(&self, request: Self::Request) -> anyhow::Result<Self::Response> {
        let subnet = SubnetID::from_str(&request.subnet)?;
        let parent = subnet.parent().ok_or_else(|| anyhow!("no parent found"))?;
        let conn = match self.pool.get(&parent) {
            None => return Err(anyhow!("target parent subnet not found")),
            Some(conn) => conn,
        };

        let subnet_config = conn.subnet();
        check_subnet(subnet_config)?;

        let from = parse_from(subnet_config, request.from)?;
        let to = request
            .to
            .map(|r| Address::from_str(&r))
            .transpose()?
            .unwrap_or(from);
        let amount = handlers::f64_to_token_amount(request.amount)?;

        conn.manager()
            .fund(subnet, subnet_config.gateway_addr(), from, to, amount)
            .await
    }
}
