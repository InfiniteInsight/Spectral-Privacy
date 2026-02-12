//! Removal execution and result tracking.

pub mod captcha;
pub mod result;
pub mod web_form;

pub use captcha::{detect_captcha, CaptchaSolver, ManualSolver};
pub use result::RemovalOutcome;
pub use web_form::WebFormSubmitter;
