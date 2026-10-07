use crate::cli::{Cli, colorize_status_text, colorize_status_text_comfy, duration, rail_table};
use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, ContentArrangement};
use futures::{SinkExt, StreamExt};
use polyfmt::{error, pause, println, resume, success};
use std::collections::VecDeque;
use std::io::{IsTerminal, Read, Write};
use termion::raw::IntoRawMode;
use tokio_tungstenite::WebSocketStream;
use tungstenite::{Message, protocol::frame::coding::CloseCode};

#[derive(Debug, Args, Clone)]
pub struct TaskSubcommands {
    #[clap(subcommand)]
    pub command: TaskCommands,

    /// Namespace Identifier.
    #[clap(long, global = true)]
    pub namespace: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TaskCommands {
    /// List all task executions.
    List {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,
    },

    /// Get details on a specific task execution.
    Get {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,
    },

    /// Attach to a running container.
    ///
    /// Gofer allows you to connect your terminal to a container and run commands.
    /// This is useful for debugging or just general informational gathering.
    ///
    /// The connection to the container only lasts as long as it is running and will
    /// be severed upon the container's completion.
    ///
    /// It should be also noted that this feature is a bit of an anti-pattern. In theory
    /// testing your containers locally + good logging + the information Gofer provides
    /// should be good enough to debug most issues. That being said there are always scheduler
    /// specific infrastructure issues that it is just helpful to have a shell within.
    Attach {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,

        /// The command to run when we first attach.
        #[arg(short, long, default_value = "/bin/sh")]
        cmd: String,
    },

    /// Cancel a specific task execution.
    ///
    /// Cancels a task execution by requesting that the scheduler gracefully stops it. Usually this means the
    /// scheduler will pass a SIGTERM to the container. If the container does not shut down within the API
    /// defined timeout or the user has passed the force flag the scheduler will then kill the container immediately.
    ///
    /// Cancelling a task execution might mean that downstream/dependent task executions are skipped.
    Cancel {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,

        /// Wait this many seconds and then force kill the container. 0 means immediately kill the container
        #[arg(short, long, default_value = "15")]
        wait_for: u64,
    },

    /// Examine logs for a particular task executions/container.
    Logs {
        /// Pipeline Identifier.
        pipeline_id: String,

        /// Run Identifier.
        run_id: u64,

        /// Task Identifier.
        task_id: String,
    },
}

impl Cli {
    pub async fn handle_task_subcommands(&self, command: TaskSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            TaskCommands::List {
                pipeline_id,
                run_id,
            } => {
                self.task_list(command.namespace, &pipeline_id, run_id)
                    .await
            }
            TaskCommands::Get {
                pipeline_id,
                run_id,
                task_id,
            } => {
                self.task_get(command.namespace, &pipeline_id, run_id, &task_id)
                    .await
            }
            TaskCommands::Attach {
                pipeline_id,
                run_id,
                task_id,
                cmd,
            } => {
                self.task_attach(command.namespace, &pipeline_id, run_id, &task_id, &cmd)
                    .await
            }
            TaskCommands::Cancel {
                pipeline_id,
                run_id,
                task_id,
                wait_for,
            } => {
                self.task_cancel(command.namespace, &pipeline_id, run_id, &task_id, wait_for)
                    .await
            }
            TaskCommands::Logs {
                pipeline_id,
                run_id,
                task_id,
            } => {
                self.task_logs(command.namespace, &pipeline_id, run_id, &task_id)
                    .await
            }
        }
    }
}

impl Cli {
    pub async fn task_list(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let tasks = self
            .client
            .list_task_executions(&namespace, pipeline_id, run_id, None)
            .await
            .context("Could not successfully retrieve tasks from Gofer api")?
            .into_inner()
            .task_executions;

        let mut table = comfy_table::Table::new();
        table
            .load_style(comfy_table::presets::ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("id")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("started")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("ended")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("duration")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("state")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
                Cell::new("status")
                    .set_alignment(CellAlignment::Center)
                    .fg(Color::Blue),
            ]);

        for task in tasks {
            table.add_row(vec![
                Cell::new(task.task_id).fg(Color::Green),
                Cell::new(
                    self.format_time(task.started)
                        .unwrap_or("Unknown".to_string()),
                ),
                Cell::new(
                    self.format_time(task.ended)
                        .unwrap_or("Still running".to_string()),
                ),
                Cell::new(duration(task.started as i64, task.ended as i64)),
                Cell::new(task.state).fg(colorize_status_text_comfy(task.state)),
                Cell::new(task.status).fg(colorize_status_text_comfy(task.status)),
            ]);
        }

        println!("{}", &table.to_string());
        Ok(())
    }

    pub async fn task_get(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task = self
            .client
            .get_task_execution(&namespace, pipeline_id, run_id, task_id, None)
            .await
            .context("Could not successfully retrieve task from Gofer api")?
            .into_inner()
            .task_execution;

        let finished = task.state == gofer_sdk::api::types::TaskExecutionState::Complete;
        let log_lines = if finished && task.started != 0 && !task.logs_removed && !task.logs_expired
        {
            self.log_tail(&namespace, pipeline_id, run_id, task_id, 5)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        let variable_rows = task
            .variables
            .into_iter()
            .map(|variable| {
                vec![
                    Cell::new(variable.key),
                    Cell::new(variable.value).fg(Color::Blue),
                    Cell::new(variable.source.to_string()).fg(Color::AnsiValue(245)),
                ]
            })
            .collect();

        const TEMPLATE: &str = r#"
  {{ vertical_line }} Parent Pipeline: {{ pipeline_id }}
  {{ run_prefix }} Parent Run: {{ run_id }}
  {{ task_prefix }} Task ID: {{ task_id }}
  {{ vertical_line }} Image: {{ image_name }}
  {%- if image_digest %}
  {{ vertical_line }} Image Digest: {{ image_digest }}
  {%- endif %}
  {{ vertical_line }} Exit Code: {{ exit_code }}
  {{ vertical_line }} Started {{ started }} and ran for {{ duration }}

  {%- if status_reason %}

  $ Status Details:
  {{ vertical_line }} Reason: {{ status_reason.reason }}
  {{ vertical_line }} Description: {{ status_reason.description }}
  {%- endif %}
  {%- if env_vars is defined %}

  $ Environment Variables:
  {%- for line in env_vars %}
  {{ line }}
  {%- endfor %}
  {%- endif %}
  {%- if log_lines %}

  $ Last {{ log_lines | length }} Log Lines:
  {%- for line in log_lines %}
  {{ vertical_line }} {{ line }}
  {%- endfor %}
  {%- endif %}

* Use '{{ task_execution_cmd }}' to view logs.
"#;

        let mut tera = tera::Tera::default();
        tera.add_raw_template("main", TEMPLATE)
            .context("Failed to render context")?;

        let mut context = tera::Context::new();
        context.insert("vertical_line", &"│".magenta().to_string());
        context.insert("pipeline_id", &task.pipeline_id.blue().to_string());
        context.insert("run_prefix", &"├─".magenta().to_string());
        context.insert("run_id", &format!("#{}", task.run_id).blue().to_string());
        context.insert("task_id", &task.task_id.blue().to_string());
        context.insert("task_prefix", &"├──".magenta().to_string());
        context.insert(
            "started",
            &self
                .format_time(task.started)
                .unwrap_or_else(|| "Not yet".to_string()),
        );
        context.insert(
            "duration",
            &duration(task.started as i64, task.ended as i64),
        );
        context.insert("image_name", &task.task.image.blue().to_string());
        context.insert("image_digest", &task.image_digest);
        context.insert("log_lines", &log_lines);
        context.insert(
            "exit_code",
            &task
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or("None".into()),
        );
        context.insert("status_reason", &task.status_reason);
        context.insert(
            "env_vars",
            &rail_table(&["KEY", "VALUE", "SOURCE"], variable_rows),
        );
        context.insert(
            "task_execution_cmd",
            &format!(
                "gofer task logs {} {} {}",
                task.pipeline_id, task.run_id, task.task_id
            )
            .cyan()
            .to_string(),
        );

        let content = tera.render("main", &context)?;
        println!(
            "  Task {} :: {} :: {}",
            task.task_id.cyan(),
            colorize_status_text(task.state),
            colorize_status_text(task.status)
        );
        println!("{}", content.trim_end());
        Ok(())
    }

    /// Returns the last `lines` lines of a task execution's log. The log stream only ends once the task is complete,
    /// so callers should only use this for finished tasks. The timeout keeps us from hanging on a log that never got
    /// its end marker, like one from an orphaned task.
    pub async fn log_tail(
        &self,
        namespace: &str,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
        lines: usize,
    ) -> Result<Vec<String>> {
        if lines == 0 {
            return Ok(vec![]);
        }

        let conn = self
            .client
            .get_logs(namespace, pipeline_id, run_id, task_id)
            .await
            .map_err(|e| anyhow!("could not get logs; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (_, mut read) = stream.split();
        let mut tail = VecDeque::with_capacity(lines);

        let read_all = async {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if tail.len() == lines {
                            tail.pop_front();
                        }
                        tail.push_back(text.trim_end_matches(['\n', '\r']).to_string());
                    }
                    Ok(Message::Close(Some(frame))) if frame.code != CloseCode::Normal => {
                        bail!("{}", frame.reason);
                    }
                    Ok(Message::Close(_))
                    | Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => break,
                    Err(e) => bail!("Error receiving logs: {}", e),
                    _ => {}
                }
            }

            Ok(())
        };

        tokio::time::timeout(std::time::Duration::from_secs(15), read_all)
            .await
            .context("Timed out while reading logs")??;

        Ok(tail.into())
    }

    pub async fn task_logs(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task_logs_conn = self
            .client
            .get_logs(&namespace, pipeline_id, run_id, task_id)
            .await
            .map_err(|e| anyhow!("could not get logs; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            task_logs_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (_, mut read) = stream.split();

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => println!("{}", text),
                Ok(Message::Binary(_)) => println!("Received binary data"),
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        match frame.code {
                            tungstenite::protocol::frame::coding::CloseCode::Normal => break,
                            _ => {
                                error!("Connection closed by server; {}", frame.reason)
                            }
                        }
                        break;
                    }
                    error!("Connection closed by server without reason");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => {
                    error!("Connection closed");
                    break;
                }
                Err(tokio_tungstenite::tungstenite::Error::Protocol(e))
                    if e.to_string()
                        .contains("Connection reset without closing handshake") =>
                {
                    error!("Connection reset without closing handshake");
                    break;
                }
                Err(e) => {
                    bail!("Error receiving message: {}", e);
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn task_cancel(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
        wait_for: u64,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        self.client
            .cancel_task_execution(&namespace, pipeline_id, run_id, task_id, wait_for)
            .await
            .context("Could not successfully cancel task")?
            .into_inner();

        success!("Successfully cancelled task '{}'", task_id);

        Ok(())
    }

    pub async fn task_attach(
        &self,
        namespace_id: Option<String>,
        pipeline_id: &str,
        run_id: u64,
        task_id: &str,
        command: &str,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let task_attach_conn = self
            .client
            .attach_task_execution(&namespace, pipeline_id, run_id, task_id, command)
            .await
            .map_err(|e| anyhow!("could not attach to task; {:#?}", e))?
            .into_inner();

        let stream = WebSocketStream::from_raw_socket(
            task_attach_conn,
            tokio_tungstenite::tungstenite::protocol::Role::Client,
            None,
        )
        .await;

        let (mut write, mut read) = stream.split();

        // Stdin is read on a plain thread because a blocked read on tokio's stdin keeps the runtime from shutting
        // down, which would leave the CLI hanging after the server ends the session.
        let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 1024];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if input_tx.blocking_send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // The session writes straight to the terminal so we pause the formatter and resume it once the session ends.
        pause!();

        // Raw mode sends each keystroke to the container as it's typed, so the container's terminal handles echo,
        // line editing, and Ctrl-C instead of ours. The guard puts the terminal back when it's dropped.
        let raw_terminal = if std::io::stdin().is_terminal() {
            match std::io::stdout().into_raw_mode() {
                Ok(raw_terminal) => Some(raw_terminal),
                Err(e) => {
                    resume!();
                    return Err(anyhow!("could not put terminal into raw mode; {}", e));
                }
            }
        } else {
            None
        };

        let result = async {
            let mut stdout = std::io::stdout();

            loop {
                tokio::select! {
                    input = input_rx.recv() => {
                        let Some(input) = input else {
                            let _ = write.send(Message::Close(None)).await;
                            return Ok(());
                        };

                        write
                            .send(Message::Binary(input.into()))
                            .await
                            .context("Error while attempting to copy user input to server")?;
                    }
                    message = read.next() => {
                        let output = match message {
                            Some(Ok(Message::Binary(bytes))) => bytes.to_vec(),
                            Some(Ok(Message::Text(text))) => text.as_bytes().to_vec(),
                            Some(Ok(Message::Close(Some(frame))))
                                if frame.code != CloseCode::Normal && !frame.reason.is_empty() =>
                            {
                                bail!("{}", frame.reason);
                            }
                            Some(Ok(Message::Close(_))) | None => return Ok(()),
                            Some(Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed)) => {
                                return Ok(());
                            }
                            Some(Err(e)) => bail!("Error receiving message: {}", e),
                            Some(Ok(_)) => continue,
                        };

                        stdout.write_all(&output)?;
                        stdout.flush()?;
                    }
                }
            }
        }
        .await;

        drop(raw_terminal);
        resume!();

        result
    }
}
