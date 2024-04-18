use crate::cli::{humanize_relative_duration, Cli};
use anyhow::{anyhow, bail, Context, Result};
use chrono::TimeZone;
use clap::{Args, Subcommand};
use futures::StreamExt;
use polyfmt::{error, print, println};
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

#[derive(Debug, Args, Clone)]
pub struct EventSubcommands {
    #[clap(subcommand)]
    pub command: EventCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum EventCommands {
    /// Get a specific event by id.
    Get { id: String },

    /// List all events.
    List {
        /// Sort events from newest to oldest
        #[arg(short, long, default_value = "false")]
        reverse: bool,

        /// Turn off showing older events. Does not work in conjunction with the reverse flag.
        #[arg(short = 'n', long, default_value = "true")]
        history: bool,
    },
}

impl Cli {
    pub async fn handle_event_subcommands(&self, command: EventSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            EventCommands::Get { id } => self.event_get(&id).await,
            EventCommands::List { reverse, history } => self.event_list(reverse, history).await,
        }
    }
}

impl Cli {
    pub async fn event_list(&self, reverse: bool, history: bool) -> Result<()> {
        let events_conn = self
            .client
            .stream_events(Some(history), Some(reverse))
            .await
            .map_err(|e| anyhow!("could not get events; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            events_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (_, mut read) = stream.split();

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    let event: gofer_sdk::api::types::Event = match serde_json::from_str(&text) {
                        Ok(event) => event,
                        Err(e) => {
                            error!("Could not serialize event; {:#?}", e);
                            continue;
                        }
                    };
                    let event_kind_str = match serde_json::to_string(&event.kind) {
                        Ok(event_kind_str) => event_kind_str,
                        Err(e) => {
                            error!("Could not serialize event kind; {:#?}", e);
                            continue;
                        }
                    };

                    let datetime = chrono::Utc
                        .timestamp_millis_opt(event.emitted as i64)
                        .unwrap();

                    println!("┌{} :: {}", datetime.to_rfc3339(), event.id);
                    println!("└::{}", event_kind_str);
                }
                Ok(Message::Binary(_)) => println!("Received binary data"),
                Ok(Message::Close(msg)) => match msg {
                    Some(msg) => {
                        bail!("Connection closed by server: {:#?}", msg);
                    }
                    None => {
                        bail!("Connection closed by server");
                    }
                },
                Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
                    bail!("Connection closed");
                }
                Err(tokio_tungstenite::tungstenite::Error::Protocol(e))
                    if e.to_string()
                        .contains("Connection reset without closing handshake") =>
                {
                    bail!("Connection reset without closing handshake");
                }
                Err(e) => {
                    bail!("Error receiving message: {}", e);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn event_get(&self, id: &str) -> Result<()> {
        let event = self
            .client
            .get_event(id)
            .await
            .context("Could not successfully retrieve event from Gofer api")?
            .into_inner()
            .event;

        const TEMPLATE: &str = r#"  [{{id}}]

  🗒 Details:
    {{kind}}

  Emitted {{emitted}}
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert(
            "emitted",
            &humanize_relative_duration(event.emitted).unwrap_or_else(|| "Unknown".to_string()),
        );
        context.insert("id", &event.id);
        context.insert("kind", &format!("{:#?}", event.kind));

        let content = tera.render("main", &context)?;
        print!("{}", content);

        Ok(())
    }
}
