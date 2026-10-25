//! Wire protocol v1 between a voice channel and the hub (PLAN.md §6).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::domain::{MessageId, SessionInfo};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ChannelFrame {
    Hello {
        protocol: u32,
        token: String,
        session: SessionInfo,
    },
    Delivered {
        id: MessageId,
    },
    Reply {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum HubFrame {
    Welcome {
        title: String,
    },
    Deliver {
        id: MessageId,
        text: String,
        #[serde(with = "time::serde::rfc3339")]
        #[ts(type = "string")]
        at: OffsetDateTime,
    },
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorCode {
    BadToken,
    BadProtocol,
    NoHello,
    Replaced,
    TooLarge,
}

impl HubFrame {
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Error {
            code,
            message: message.into(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("hub frames always serialise")
    }
}
