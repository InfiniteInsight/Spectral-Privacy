//! Web form removal submission.

use crate::definition::{BrokerDefinition, RemovalMethod};
use crate::error::{BrokerError, Result};
use crate::removal::{detect_captcha, CaptchaSolver, ManualSolver, RemovalOutcome};
use spectral_browser::{BrowserActions, BrowserEngine};
use std::collections::HashMap;

/// Web form submitter for automated opt-out requests.
pub struct WebFormSubmitter {
    engine: BrowserEngine,
    #[allow(dead_code)]
    captcha_solver: Box<dyn CaptchaSolver>,
}

impl WebFormSubmitter {
    /// Create a new web form submitter.
    pub async fn new() -> Result<Self> {
        let engine = BrowserEngine::new()
            .await
            .map_err(|e| BrokerError::RemovalError {
                broker_id: "unknown".to_string(),
                reason: format!("Failed to create browser engine: {e}"),
            })?;

        Ok(Self {
            engine,
            captcha_solver: Box::new(ManualSolver),
        })
    }

    /// Submit a removal request for a broker.
    pub async fn submit(
        &self,
        broker_def: &BrokerDefinition,
        field_values: HashMap<String, String>,
    ) -> Result<RemovalOutcome> {
        // Extract removal configuration
        let RemovalMethod::WebForm {
            url,
            form_selectors,
            ..
        } = &broker_def.removal
        else {
            return Err(BrokerError::RemovalError {
                broker_id: broker_def.id().to_string(),
                reason: "Not a web-form removal method".to_string(),
            });
        };

        // Navigate to opt-out form
        self.engine
            .navigate(url)
            .await
            .map_err(|e| BrokerError::RemovalError {
                broker_id: broker_def.id().to_string(),
                reason: format!("Navigation failed: {e}"),
            })?;

        // Check for CAPTCHA
        let captcha_detected =
            detect_captcha(&self.engine, form_selectors.captcha_frame.as_deref()).await?;

        if captcha_detected {
            return Ok(RemovalOutcome::RequiresCaptcha {
                captcha_url: url.clone(),
            });
        }

        // Fill form fields
        for (field_name, value) in &field_values {
            let selector = match field_name.as_str() {
                "listing_url" => &form_selectors.listing_url_input,
                "email" => &form_selectors.email_input,
                "first_name" => &form_selectors.first_name_input,
                "last_name" => &form_selectors.last_name_input,
                _ => continue,
            };

            if let Some(sel) = selector {
                self.engine.fill_field(sel, value).await.map_err(|e| {
                    BrokerError::RemovalError {
                        broker_id: broker_def.id().to_string(),
                        reason: format!("Failed to fill field {field_name}: {e}"),
                    }
                })?;
            }
        }

        // Submit form
        self.engine
            .click(&form_selectors.submit_button)
            .await
            .map_err(|e| BrokerError::RemovalError {
                broker_id: broker_def.id().to_string(),
                reason: format!("Failed to click submit: {e}"),
            })?;

        // Wait a moment for submission to process
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Check for success indicator
        if let Some(success_sel) = &form_selectors.success_indicator {
            match self.engine.wait_for_selector(success_sel, 5000).await {
                Ok(()) => {
                    // Success! Get email from field_values if present
                    let email = field_values.get("email").cloned().unwrap_or_default();

                    return Ok(RemovalOutcome::RequiresEmailVerification {
                        email: email.clone(),
                        sent_to: email,
                    });
                }
                Err(_) => {
                    // Success indicator not found - might have failed
                    return Ok(RemovalOutcome::Failed {
                        reason: "Success confirmation not detected".to_string(),
                        error_details: None,
                    });
                }
            }
        }

        // No success indicator configured, assume submitted
        Ok(RemovalOutcome::Submitted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_form_submitter_struct() {
        // Just verify the struct compiles
        assert_eq!(std::mem::size_of::<Box<dyn CaptchaSolver>>(), 16);
    }
}
