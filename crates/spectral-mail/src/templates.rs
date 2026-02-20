use std::collections::HashMap;

pub struct EmailTemplate {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Substitutes `{{field_name}}` placeholders in template with profile values.
pub fn render_template(
    template: &str,
    email: &str,
    to: &str,
    profile_fields: &HashMap<String, String>,
) -> EmailTemplate {
    let subject = format!(
        "Opt-Out Request — {}",
        profile_fields.get("full_name").cloned().unwrap_or_default()
    );
    let mut body = template.to_string();
    for (key, value) in profile_fields {
        body = body.replace(&format!("{{{{{key}}}}}"), value);
    }
    // Replace remaining known placeholders
    body = body.replace("{{email}}", email);
    EmailTemplate {
        to: to.to_string(),
        subject,
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_substitutes_fields() {
        let mut fields = HashMap::new();
        fields.insert("full_name".to_string(), "Alice Smith".to_string());
        fields.insert("address".to_string(), "123 Main St".to_string());
        let template = "Name: {{full_name}}\nAddress: {{address}}\nEmail: {{email}}";
        let result = render_template(template, "alice@example.com", "optout@broker.com", &fields);
        assert_eq!(result.to, "optout@broker.com");
        assert!(result.subject.contains("Alice Smith"));
        assert!(result.body.contains("Alice Smith"));
        assert!(result.body.contains("123 Main St"));
        assert!(result.body.contains("alice@example.com"));
    }
}
