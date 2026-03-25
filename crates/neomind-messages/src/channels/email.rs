//! Email notification channel.

#[cfg(feature = "email")]
use async_trait::async_trait;

#[cfg(feature = "email")]
use super::super::{Error, Message, Result};
#[cfg(feature = "email")]
use super::MessageChannel;

/// Email channel for sending messages via SMTP.
#[cfg(feature = "email")]
#[derive(Debug, Clone)]
pub struct EmailChannel {
    name: String,
    enabled: bool,
    smtp_server: String,
    smtp_port: u16,
    username: String,
    password: String,
    from_address: String,
    to_addresses: Vec<String>,
    use_tls: bool,
}

#[cfg(feature = "email")]
impl EmailChannel {
    pub fn new(
        name: String,
        smtp_server: String,
        smtp_port: u16,
        username: String,
        password: String,
        from_address: String,
    ) -> Self {
        Self {
            name,
            enabled: true,
            smtp_server,
            smtp_port,
            username,
            password,
            from_address,
            to_addresses: Vec::new(),
            use_tls: true,
        }
    }

    pub fn add_recipient(mut self, address: String) -> Self {
        self.to_addresses.push(address);
        self
    }

    pub fn with_recipients(mut self, addresses: Vec<String>) -> Self {
        self.to_addresses = addresses;
        self
    }

    /// Set recipients (for updating after creation)
    pub fn set_recipients_internal(&mut self, addresses: Vec<String>) {
        self.to_addresses = addresses;
    }

    /// Get current recipients
    pub fn get_recipients_internal(&self) -> &[String] {
        &self.to_addresses
    }

    pub fn without_tls(mut self) -> Self {
        self.use_tls = false;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    fn build_email_body(&self, message: &Message) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {{ font-family: Arial, sans-serif; }}
        .message {{ padding: 20px; border-radius: 5px; }}
        .severity-info {{ background-color: #d4edda; border-left: 4px solid #28a745; }}
        .severity-warning {{ background-color: #fff3cd; border-left: 4px solid #ffc107; }}
        .severity-critical {{ background-color: #f8d7da; border-left: 4px solid #dc3545; }}
        .severity-emergency {{ background-color: #f5c6cb; border-left: 4px solid #bd2130; }}
        .timestamp {{ color: #6c757d; font-size: 0.9em; }}
        .source {{ font-weight: bold; }}
    </style>
</head>
<body>
    <div class="message severity-{}">
        <h2>{}</h2>
        <p class="timestamp">时间: {}</p>
        <p><strong>来源:</strong> <span class="source">{}</span></p>
        <p><strong>消息:</strong> {}</p>
    </div>
</body>
</html>"#,
            message.severity.as_str(),
            message.title,
            message.timestamp.format("%Y-%m-%d %H:%M:%S"),
            message.source,
            message.message
        )
    }
}

#[cfg(feature = "email")]
#[async_trait]
impl MessageChannel for EmailChannel {
    fn name(&self) -> &str {
        &self.name
    }

    fn channel_type(&self) -> &str {
        "email"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn send(&self, message: &Message) -> Result<()> {
        if !self.enabled {
            return Err(Error::ChannelDisabled(self.name.clone()));
        }

        if self.to_addresses.is_empty() {
            return Err(Error::SendFailed("No recipients configured".to_string()));
        }

        let html_body = self.build_email_body(message);
        let subject = format!("[{}] {}", message.severity, message.title);

        let from_mailbox: lettre::message::Mailbox = self
            .from_address
            .parse()
            .map_err(|e| Error::InvalidConfiguration(format!("Invalid from address: {}", e)))?;

        let mut email_builder = lettre::Message::builder()
            .from(from_mailbox.clone())
            .subject(subject);

        for to_addr in &self.to_addresses {
            let mailbox: lettre::message::Mailbox = to_addr
                .parse()
                .map_err(|e| Error::InvalidConfiguration(format!("Invalid to address: {}", e)))?;
            email_builder = email_builder.to(mailbox);
        }

        let email = email_builder
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_PLAIN)
                            .body(format!("{}\n\n{}", message.title, message.message)),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_HTML)
                            .body(html_body),
                    ),
            )
            .map_err(|e| Error::SendFailed(format!("Failed to build email: {}", e)))?;

        let smtp_server = self.smtp_server.clone();
        let smtp_port = self.smtp_port;
        let username = self.username.clone();
        let password = self.password.clone();

        tokio::task::spawn_blocking(move || {
            let creds =
                lettre::transport::smtp::authentication::Credentials::new(username, password);

            use lettre::transport::smtp::client::Tls;
            use lettre::transport::smtp::client::TlsParametersBuilder;

            let tls_params = TlsParametersBuilder::new(smtp_server.clone())
                .build()
                .map_err(|e| Error::SendFailed(format!("Failed to build TLS params: {}", e)))?;

            let mailer = lettre::SmtpTransport::relay(&smtp_server)
                .map_err(|e| Error::SendFailed(format!("Invalid SMTP server: {}", e)))?
                .port(smtp_port)
                .tls(Tls::Required(tls_params))
                .credentials(creds)
                .build();

            lettre::Transport::send(&mailer, &email)
                .map_err(|e| Error::SendFailed(format!("Failed to send email: {}", e)))?;

            Ok::<(), Error>(())
        })
        .await
        .map_err(|e| Error::SendFailed(format!("Task join error: {}", e)))?
    }

    fn set_recipients(&mut self, recipients: Vec<String>) {
        self.to_addresses = recipients;
    }

    fn get_recipients(&self) -> Vec<String> {
        self.to_addresses.clone()
    }
}

/// Factory for creating email channels.
#[cfg(feature = "email")]
pub struct EmailChannelFactory;

#[cfg(feature = "email")]
impl super::ChannelFactory for EmailChannelFactory {
    fn channel_type(&self) -> &str {
        "email"
    }

    fn create(&self, config: &serde_json::Value) -> Result<std::sync::Arc<dyn MessageChannel>> {
        let smtp_server = config
            .get("smtp_server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidConfiguration("Missing smtp_server".to_string()))?;

        // Support both number and string types for smtp_port
        let smtp_port = config
            .get("smtp_port")
            .and_then(|v| {
                if let Some(n) = v.as_u64() {
                    Some(n)
                } else if let Some(s) = v.as_str() {
                    s.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| Error::InvalidConfiguration("Missing or invalid smtp_port".to_string()))?
            as u16;

        let username = config
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidConfiguration("Missing username".to_string()))?
            .to_string();

        let password = config
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidConfiguration("Missing password".to_string()))?
            .to_string();

        let from_address = config
            .get("from_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidConfiguration("Missing from_address".to_string()))?
            .to_string();

        let name = config
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("email")
            .to_string();

        let mut channel = EmailChannel::new(
            name,
            smtp_server.to_string(),
            smtp_port,
            username,
            password,
            from_address,
        );

        if let Some(recipients) = config.get("recipients") {
            if let Some(arr) = recipients.as_array() {
                for addr in arr {
                    if let Some(str_addr) = addr.as_str() {
                        channel = channel.add_recipient(str_addr.to_string());
                    }
                }
            }
        }

        if !config
            .get("use_tls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
        {
            channel = channel.without_tls();
        }

        if !config
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
        {
            channel = channel.disabled();
        }

        Ok(std::sync::Arc::new(channel))
    }
}
