use crate::gateway::route::SharedGateway;

/// Global server state — just holds the gateway
/// Individual world state lives in WorldInstance
pub(crate) struct GlobalServer {
    pub gateway: SharedGateway,
}

impl GlobalServer {
    pub fn new(gateway: SharedGateway) -> Self {
        Self { gateway }
    }
}