//! Crate error type.

use thiserror::Error;

/// Errors produced while building or parsing ECR17 messages (and, under the
/// `tokio-transport` feature, while talking to a terminal).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Ecr17Error {
    /// A value is longer than its fixed-width field. ECR17 fields have a fixed
    /// length, so an oversized value would shift every following field and corrupt
    /// the frame — building it is refused.
    #[error("ECR17: value '{value}' exceeds fixed field width of {width} bytes")]
    FieldOverflow {
        /// The offending value.
        value: String,
        /// The field width it overflowed.
        width: usize,
    },

    /// A monetary amount was negative.
    #[error("ECR17: amount must be non-negative")]
    NegativeAmount,

    /// A VAS request payload exceeded the 1024-byte limit.
    #[error("ECR17: VAS request exceeds 1024 bytes")]
    VasTooLong,

    /// The additional-data TAG content was empty or longer than 100 chars.
    #[error("ECR17: additional TAG content must be 1..=100 chars")]
    TagContentInvalid,

    /// The tokenization contract code was empty or longer than 18 chars.
    #[error("ECR17: tokenization contract code must be 1..=18 chars")]
    ContractCodeInvalid,
}

/// Convenience result alias.
pub type Result<T> = core::result::Result<T, Ecr17Error>;
