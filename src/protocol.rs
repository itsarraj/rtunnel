use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Sent by the client upon connecting to the control port.
    Register,
    /// Sent by the server to instruct the client to open a new data connection for an incoming request.
    Open { id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataMessage {
    /// Sent by the client over a newly established data connection to associate it with an `Open` request.
    Bind { id: Uuid },
}
