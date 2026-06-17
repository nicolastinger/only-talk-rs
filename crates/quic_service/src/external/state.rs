use std::fmt;

/// Service lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Uninitialized,
    Initializing,
    Running,
    Stopping,
    Stopped,
}

impl ServiceState {
    /// Check if transition to target state is allowed
    pub fn can_transition_to(&self, target: ServiceState) -> bool {
        matches!(
            (self, target),
            (ServiceState::Uninitialized, ServiceState::Initializing)
                | (ServiceState::Initializing, ServiceState::Running)
                | (ServiceState::Initializing, ServiceState::Stopping)
                | (ServiceState::Running, ServiceState::Stopping)
                | (ServiceState::Stopping, ServiceState::Stopped)
        )
    }

    /// Perform state transition, returns error for invalid transitions
    pub fn transition_to(&mut self, target: ServiceState) -> Result<(), ServiceError> {
        if self.can_transition_to(target) {
            *self = target;
            Ok(())
        } else {
            Err(ServiceError::InvalidStateTransition { from: *self, to: target })
        }
    }
}

/// Service error types
#[derive(Debug)]
pub enum ServiceError {
    InvalidStateTransition { from: ServiceState, to: ServiceState },
    Config(String),
    Runtime(anyhow::Error),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Invalid state transition: cannot transition from {:?} to {:?}", from, to)
            }
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Runtime(err) => write!(f, "Service runtime error: {}", err),
        }
    }
}

impl std::error::Error for ServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Runtime(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ServiceError {
    fn from(err: anyhow::Error) -> Self {
        ServiceError::Runtime(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        assert!(ServiceState::Uninitialized.can_transition_to(ServiceState::Initializing));
        assert!(ServiceState::Initializing.can_transition_to(ServiceState::Running));
        assert!(ServiceState::Initializing.can_transition_to(ServiceState::Stopping));
        assert!(ServiceState::Running.can_transition_to(ServiceState::Stopping));
        assert!(ServiceState::Stopping.can_transition_to(ServiceState::Stopped));
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(!ServiceState::Uninitialized.can_transition_to(ServiceState::Running));
        assert!(!ServiceState::Uninitialized.can_transition_to(ServiceState::Stopped));
        assert!(!ServiceState::Running.can_transition_to(ServiceState::Initializing));
        assert!(!ServiceState::Stopped.can_transition_to(ServiceState::Running));
        assert!(!ServiceState::Stopped.can_transition_to(ServiceState::Stopping));
    }

    #[test]
    fn test_transition_to() {
        let mut state = ServiceState::Uninitialized;
        assert!(state.transition_to(ServiceState::Initializing).is_ok());
        assert_eq!(state, ServiceState::Initializing);

        assert!(state.transition_to(ServiceState::Running).is_ok());
        assert_eq!(state, ServiceState::Running);

        let result = state.transition_to(ServiceState::Uninitialized);
        assert!(result.is_err());
        assert_eq!(state, ServiceState::Running);
    }
}
