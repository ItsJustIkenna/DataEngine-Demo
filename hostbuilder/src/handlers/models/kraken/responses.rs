use anyhow::{
    anyhow,
    Result
};
use serde::{
    Deserialize,
    Serialize
};
use serde_json::Value;

use super::token_response::TokenResponse;
use super::status::Status;
use super::level3_subscribe::Level3Subscribe;
use super::level3_unsubscribe::Level3Unsubscribe;
use super::level3_sub_ack::Level3SubscribeAcknowledgement;
use super::level3_unsub_ack::Level3UnsubscribeAcknowledgement;
use super::heartbeat::HeartBeat;
use super::trade_subscribe::TradeSubscribe;
use super::trade_unsubscribe::TradeUnsubscribe;
use super::trade_sub_ack::TradeSubscribeAcknowledgement;
use super::level3_snapshot::Level3Snapshot;
use super::level3_update::Level3Update;
use super::trade_unsub_ack::TradeUnsubscribeAcknowledgement;
use super::trade_snapshot::TradeSnapshot;
use super::trade_update::TradeUpdate;

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    TokenResponse(TokenResponse),
    Status(Status),
    Level3Subscribe(Level3Subscribe),
    Level3Unsubscribe(Level3Unsubscribe),
    Level3SubscribeAck(Level3SubscribeAcknowledgement),
    Level3UnsubscribeAck(Level3UnsubscribeAcknowledgement),
    Level3Snapshot(Level3Snapshot),
    Level3Update(Level3Update),
    HeartBeat(HeartBeat),
    TradeSubscribe(TradeSubscribe),
    TradeUnsubscribe(TradeUnsubscribe),
    TradeSubscribeAck(TradeSubscribeAcknowledgement),
    TradeUnsubscribeAck(TradeUnsubscribeAcknowledgement),
    TradeSnapshot(TradeSnapshot),
    TradeUpdate(TradeUpdate),
}

pub fn parse_message(msg: &str) -> Result<Response> {
    // First, parse the string into a serde_json::Value
    let json_msg: Value =
        serde_json::from_str(msg).map_err(|e| anyhow!("Failed to parse message: {}", e))?;

    if json_msg.get("result").is_some() && json_msg["result"].get("token").is_some() {
        let token_response = TokenResponse::from_json(&json_msg)?;
        Ok(Response::TokenResponse(token_response))
    } else if json_msg.get("channel").is_some() && json_msg["channel"] == "status" {
        let status = Status::from_json(&json_msg)?;
        Ok(Response::Status(status))
    } else if json_msg.get("channel").is_some() && json_msg["channel"] == "heartbeat" {
        let heartbeat = HeartBeat::from_json(&json_msg)?;
        Ok(Response::HeartBeat(heartbeat))
    } else if json_msg.get("method").is_some()
        && json_msg["method"] == "subscribe"
        && json_msg.get("params").is_some()
    {
        let subscribe = Level3Subscribe::from_json(&json_msg)?;
        Ok(Response::Level3Subscribe(subscribe))
    } else if json_msg.get("method").is_some()
        && json_msg["method"] == "subscribe"
        && json_msg.get("result").is_some()
    {
        let subscribe_ack = Level3SubscribeAcknowledgement::from_json(&json_msg)?;
        Ok(Response::Level3SubscribeAck(subscribe_ack))
    } else if json_msg.get("method").is_some()
        && json_msg["method"] == "unsubscribe"
        && json_msg.get("params").is_some()
    {
        let unsubscribe = Level3Unsubscribe::from_json(&json_msg)?;
        Ok(Response::Level3Unsubscribe(unsubscribe))
    } else if json_msg.get("method").is_some()
        && json_msg["method"] == "unsubscribe"
        && json_msg.get("result").is_some()
    {
        let unsubscribe_ack = Level3UnsubscribeAcknowledgement::from_json(&json_msg)?;
        Ok(Response::Level3UnsubscribeAck(unsubscribe_ack))
    } else if json_msg.get("channel").is_some()
        && json_msg["channel"] == "level3"
        && json_msg.get("type").is_some()
        && json_msg["type"] == "snapshot"
    {
        let snapshot = Level3Snapshot::from_json(&json_msg)?;
        Ok(Response::Level3Snapshot(snapshot))
    } else if json_msg.get("channel").is_some()
        && json_msg["channel"] == "level3"
        && json_msg.get("type").is_some()
        && json_msg["type"] == "update"
    {
        let update = Level3Update::from_json(&json_msg)?;
        Ok(Response::Level3Update(update))
    } else if json_msg.get("channel").is_some()
        && json_msg["channel"] == "trade"
        && json_msg.get("type").is_some()
        && json_msg["type"] == "snapshot"
    {
        let trade = TradeSnapshot::from_json(&json_msg)?;
        Ok(Response::TradeSnapshot(trade))
    } else if json_msg.get("channel").is_some()
        && json_msg["channel"] == "trade"
        && json_msg.get("type").is_some()
        && json_msg["type"] == "update"
    {
        let update = TradeUpdate::from_json(&json_msg)?;
        Ok(Response::TradeUpdate(update))
    } else {
        Err(anyhow!("Failed to parse message: {:?}", json_msg))
    }
}