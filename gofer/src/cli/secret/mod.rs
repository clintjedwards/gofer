mod global;
mod pipeline;

use crate::cli::Cli;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use polyfmt::{pause, resume};
use std::io::{IsTerminal, Read, Write};
use termion::input::TermRead;

/// Works out the secret value for the put commands. A value passed on the command line is used as is. Otherwise
/// (no value, or '@') we read stdin: piped input is read in full so whole files work, and a terminal gets a prompt
/// with the typing hidden unless `echo` is set.
fn resolve_secret(secret: Option<String>, echo: bool) -> Result<String> {
    if let Some(secret) = secret.filter(|secret| secret != "@") {
        return Ok(secret);
    }

    let mut stdin = std::io::stdin();
    let input = if stdin.is_terminal() {
        // The spinner redraws its line constantly and would scribble over the prompt, so stop it while we ask.
        pause!();
        let input = prompt_secret(&mut stdin, echo);
        resume!();
        input?
    } else {
        let mut content = String::new();
        stdin
            .read_to_string(&mut content)
            .context("Could not read secret from stdin")?;
        content
    };

    // Both 'echo value |' and pressing Enter leave a trailing newline that would otherwise end up in the secret
    // and quietly break whatever uses it, like a URL handed to curl.
    let secret = input.trim_end_matches(['\r', '\n']).to_string();
    if secret.is_empty() {
        bail!("Secret is empty");
    }

    Ok(secret)
}

fn prompt_secret(stdin: &mut std::io::Stdin, echo: bool) -> Result<String> {
    eprint!("Secret: ");
    std::io::stderr().flush()?;

    if echo {
        let mut line = String::new();
        // Spelled out because termion's TermRead also has a read_line.
        std::io::Stdin::read_line(stdin, &mut line)
            .context("Could not read secret from terminal")?;
        return Ok(line);
    }

    // read_passwd needs a handle to the terminal itself to switch it into raw mode, which is what stops the typed
    // characters from being shown.
    let mut tty = termion::get_tty().context("Could not open terminal")?;
    let input = stdin
        .read_passwd(&mut tty)
        .context("Could not read secret from terminal")?;
    // Raw mode swallows the Enter key, so move past the prompt ourselves.
    eprintln!();
    input.context("Secret entry cancelled")
}

#[derive(Debug, Args, Clone)]
pub struct SecretSubcommands {
    #[clap(subcommand)]
    pub command: SecretCommands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum SecretCommands {
    /// Manage global secrets.
    ///
    /// Gofer allows you to store global secrets. These secrets are then used to populate all the place where
    /// Gofer needs to use shared secrets. Only accessible to admins.
    Global(global::GlobalSecretSubcommands),

    /// Manage pipeline secrets.
    ///
    /// Gofer allows you to store pipeline secrets. These secrets are then used to populate wherever variables are used.
    Pipeline(pipeline::PipelineSecretSubcommands),
}

impl Cli {
    pub async fn handle_secret_subcommands(&self, command: SecretSubcommands) -> Result<()> {
        let cmds = command.command;
        match cmds {
            SecretCommands::Global(secret) => self.handle_global_secret_subcommands(secret).await,
            SecretCommands::Pipeline(secret) => {
                self.handle_pipeline_secret_subcommands(secret).await
            }
        }
    }
}
