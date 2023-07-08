use anyerror::AnyError;

/// Error variants related to configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("ParseError: {source} while parsing ({args:?})")]
    ParseError { source: AnyError, args: Vec<String> },

    /// The min election timeout is not smaller than the max election timeout.
    #[error("election timeout: min({min}) must be < max({max})")]
    ElectionTimeout { min: u64, max: u64 },

    #[error("max_payload_entries must be > 0")]
    MaxPayloadIs0,

    #[error("election_timeout_min({election_timeout_min}) must be > heartbeat_interval({heartbeat_interval})")]
    ElectionTimeoutLTHeartBeat {
        election_timeout_min: u64,
        heartbeat_interval: u64,
    },

    #[error("snapshot policy string is invalid: '{invalid:?}' expect: '{syntax}'")]
    InvalidSnapshotPolicy { invalid: String, syntax: String },

    #[error("{reason} when parsing {invalid:?}")]
    InvalidNumber { invalid: String, reason: String },
}
