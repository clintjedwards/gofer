mod get;
mod install;
mod list;
mod uninstall;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct TriggerSubcommands {
    #[clap(subcommand)]
    pub command: TriggerCommands,
}

#[derive(Debug, Subcommand)]
pub enum TriggerCommands {
    /// List triggers.
    List,

    /// Detail trigger by name.
    Get { name: String },

    /// Install a trigger by image.
    ///
    /// Triggers sometimes contain their own installation scripts to make installation a bit easier.
    /// To aid in this:
    ///   1) Gofer attempts to run the trigger container locally and connects the user's terminal to
    ///   the stdout/in/err.
    ///   2) The trigger container will walk the user through the installation steps required for the trigger.
    ///   3) The trigger container will attempt to install the trigger into Gofer on behalf of the user.
    Install {
        /// Custom name of trigger.
        name: String,

        /// The container image address.
        image: String,

        /// URL of Gofer server
        #[clap(long)]
        host: Option<String>,

        /// The username needed for auth to the container repository.
        #[clap(short, long)]
        user: Option<String>,

        /// The password needed for auth to the container repository.
        #[clap(short, long)]
        pass: Option<String>,
    },

    /// Uninstall a trigger by name.
    Uninstall { name: String },
}
