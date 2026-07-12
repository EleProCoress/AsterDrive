//! 产品邮件投递入口。

use std::sync::Arc;

use chrono::Utc;

use crate::config::RuntimeConfig;
use crate::config::{mail, site_url};
use crate::errors::Result;
use crate::runtime::MailRuntimeState;
use crate::utils::id;
use aster_forge_mail::{MailRecipient, MailSender, RenderedMail};

pub fn runtime_sender(runtime_config: Arc<RuntimeConfig>) -> Arc<dyn MailSender> {
    aster_forge_mail::smtp_sender(move || mail::runtime_mail_settings(&runtime_config))
}

pub async fn send_rendered(
    state: &impl MailRuntimeState,
    to: MailRecipient,
    rendered: RenderedMail,
) -> Result<()> {
    send_rendered_with(state.runtime_config(), state.mail_sender(), to, rendered).await
}

pub async fn send_rendered_with(
    runtime_config: &RuntimeConfig,
    mail_sender: &Arc<dyn MailSender>,
    to: MailRecipient,
    rendered: RenderedMail,
) -> Result<()> {
    let settings = mail::runtime_mail_settings(runtime_config);
    aster_forge_mail::send_rendered_with(mail_sender, &settings, to, rendered)
        .await
        .map_err(Into::into)
}

pub async fn send_test_email(
    state: &impl MailRuntimeState,
    email: &str,
    triggered_by: Option<&str>,
) -> Result<()> {
    let timestamp = Utc::now().to_rfc3339();
    let site_url = site_url::public_site_url(state.runtime_config())
        .unwrap_or_else(|| "(not configured)".to_string());
    let triggered_by = triggered_by.unwrap_or("admin");
    tracing::debug!(
        to = %email,
        triggered_by = %triggered_by,
        "mail: building test email"
    );

    send_rendered(
        state,
        MailRecipient {
            address: email.to_string(),
            display_name: None,
        },
        RenderedMail {
            subject: "AsterDrive SMTP test".to_string(),
            text_body: format!(
                "This is a test email from AsterDrive.\n\nTriggered by: {triggered_by}\nSent at (UTC): {timestamp}\nPublic site URL: {site_url}\n\nIf you received this email, your SMTP settings are working."
            ),
            html_body: format!(
                "<p>This is a test email from AsterDrive.</p><p><strong>Triggered by:</strong> {triggered_by}<br /><strong>Sent at (UTC):</strong> {timestamp}<br /><strong>Public site URL:</strong> {site_url}</p><p>If you received this email, your SMTP settings are working.</p>"
            ),
        },
    )
    .await
}

pub fn build_verification_token() -> String {
    format!("cv_{}", id::new_short_token())
}
