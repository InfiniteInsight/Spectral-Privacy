//! Removal execution and result tracking.

pub mod captcha;
pub mod result;

pub use captcha::{detect_captcha, CaptchaSolver, ManualSolver};
pub use result::RemovalOutcome;
