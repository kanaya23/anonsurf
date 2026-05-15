use crate::{AnonStatus, BridgeMode, DnsBackend, FirewallBackend, Status, TorStatus};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OperationError {
    #[error("operation already in progress")]
    Busy,
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    status: Status,
    logs: VecDeque<String>,
    max_logs: usize,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self {
            status: Status::default(),
            logs: VecDeque::with_capacity(256),
            max_logs: 256,
        }
    }
}

impl ServiceState {
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn logs(&self, limit: usize) -> Vec<String> {
        let start = self.logs.len().saturating_sub(limit);
        self.logs.iter().skip(start).cloned().collect()
    }

    pub fn log(&mut self, line: impl Into<String>) {
        if self.logs.len() == self.max_logs {
            self.logs.pop_front();
        }
        self.logs.push_back(line.into());
    }

    pub fn begin_transition(&mut self, message: impl Into<String>) -> Result<(), OperationError> {
        if self.status.status == AnonStatus::Transitioning {
            return Err(OperationError::Busy);
        }
        self.status.status = AnonStatus::Transitioning;
        self.status.last_error = None;
        self.log(message);
        Ok(())
    }

    pub fn mark_enabled(
        &mut self,
        tor_status: TorStatus,
        dns_backend: DnsBackend,
        firewall_backend: FirewallBackend,
        bridge_mode: BridgeMode,
    ) {
        self.status.status = AnonStatus::Enabled;
        self.status.tor_status = tor_status;
        self.status.dns_backend = dns_backend;
        self.status.firewall_backend = firewall_backend;
        self.status.bridge_mode = bridge_mode;
        self.status.last_error = None;
        self.log("service enabled");
    }

    pub fn mark_disabled(&mut self) {
        self.status.status = AnonStatus::Disabled;
        self.status.tor_status = TorStatus::Stopped;
        self.status.current_exit_ip = None;
        self.status.is_tor = None;
        self.status.firewall_backend = FirewallBackend::Unknown;
        self.status.last_error = None;
        self.log("service disabled");
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        let error = error.into();
        self.status.status = AnonStatus::Failed;
        self.status.tor_status = TorStatus::Failed;
        self.status.last_error = Some(error.clone());
        self.log(format!("failed: {error}"));
    }

    pub fn update_tor_check(&mut self, ip: Option<String>, is_tor: bool) {
        self.status.current_exit_ip = ip;
        self.status.is_tor = Some(is_tor);
        self.log(format!("tor check: is_tor={is_tor}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_rejects_nested_operation() {
        let mut state = ServiceState::default();
        state.begin_transition("start").unwrap();
        assert_eq!(state.begin_transition("stop"), Err(OperationError::Busy));
    }

    #[test]
    fn failed_state_keeps_last_error() {
        let mut state = ServiceState::default();
        state.begin_transition("start").unwrap();
        state.mark_failed("tor did not bootstrap");
        let status = state.status();
        assert_eq!(status.status, AnonStatus::Failed);
        assert_eq!(status.last_error.as_deref(), Some("tor did not bootstrap"));
    }
}
