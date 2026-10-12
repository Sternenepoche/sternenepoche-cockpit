//! Optional mail transport. Disabled by default; no code is exposed by the API.
use lettre::{message::header::ContentType, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MailConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub security: String,
    pub username: String,
    pub password: String,
    pub from: String,
}
impl MailConfig {
    pub fn initialize(root: &Path) -> Result<(), String> {
        let path=root.join("mail-private.json");
        if !path.exists() {
            let config=Self {port:587,security:"starttls".into(),..Self::default()};
            let data=serde_json::to_vec_pretty(&config).map_err(|_|"Mailkonfiguration nicht serialisierbar")?;
            super::write_private(&path,&data).map_err(|_|"Mailkonfiguration nicht gespeichert")?;
        }
        Ok(())
    }
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let data=std::fs::read(root.join("mail-private.json")).map_err(|_|"Mailkonfiguration fehlt")?;
        if data.len()>16384 { return Err("Mailkonfiguration zu groß".into()); }
        let config:Self=serde_json::from_slice(&data).map_err(|_|"Mailkonfiguration ungültig")?;
        if !config.enabled { return Ok(None); }
        if config.port==0 || config.host.is_empty() || config.from.parse::<lettre::message::Mailbox>().is_err() {
            return Err("SMTP-Host, Port und Absender prüfen".into());
        }
        if !matches!(config.security.as_str(),"tls"|"starttls"|"local_test") || (config.security=="local_test" && !matches!(config.host.as_str(),"127.0.0.1"|"localhost"|"::1")) {
            return Err("SMTP benötigt TLS oder STARTTLS; Testtransport ausschließlich lokal".into());
        }
        Ok(Some(config))
    }
    pub fn send_code(&self,email:&str,code:&str)->Result<(),String> {
        let message=Message::builder()
            .from(self.from.parse().map_err(|_|"Absender ungültig")?)
            .to(email.parse().map_err(|_|"E-Mail-Adresse ungültig")?)
            .subject("Dein Sternenepoche-Bestätigungscode")
            .header(ContentType::TEXT_PLAIN)
            .body(format!("{code}\n\nDein Bestätigungscode für Sternenepoche.\nEr ist 10 Minuten lang gültig und kann einmal verwendet werden.\n\nDu hast keine Anmeldung begonnen? Dann ignoriere diese Nachricht.\n"))
            .map_err(|_|"Nachricht konnte nicht erstellt werden")?;
        let builder=match self.security.as_str() {
            "tls"=>SmtpTransport::relay(&self.host),
            "starttls"=>SmtpTransport::starttls_relay(&self.host),
            "local_test" if matches!(self.host.as_str(),"127.0.0.1"|"localhost"|"::1")=>Ok(SmtpTransport::builder_dangerous(&self.host)),
            _=>return Err("Unsicherer SMTP-Transport abgelehnt".into()),
        }.map_err(|_|"SMTP-Verbindung nicht konfigurierbar")?;
        let mut builder=builder.port(self.port).timeout(Some(Duration::from_secs(10)));
        if !self.username.is_empty() { builder=builder.credentials(Credentials::new(self.username.clone(),self.password.clone())); }
        // Do not include provider errors: they may contain private connection details.
        builder.build().send(&message).map(|_|()).map_err(|_|"E-Mail konnte nicht versendet werden. Später erneut versuchen.".into())
    }
}
