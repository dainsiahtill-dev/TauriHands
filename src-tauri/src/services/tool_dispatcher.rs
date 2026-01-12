use serde::{Deserialize, Serialize};

use crate::services::kernel::{Action, Observation};

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub allow_network: bool,
    pub command_policy: String,
    pub path_policy: String,
}

pub trait ToolDispatcher {
    fn dispatch(
        &self,
        action: &Action,
        session_id: Option<String>,
        on_chunk: &mut dyn FnMut(String),
    ) -> Result<Observation, String>;
}
