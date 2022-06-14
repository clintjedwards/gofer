use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::process;

/// Convenience wrapper trait meant for use around the indicatif::ProgressBar type
/// Needed because for some reason calling finish_with_message doesn't
/// clear the progress of the spinner and leaves it in whatever the current state
/// it was in.
///
/// Also allows easy access to the red error and green checkmark finish prefixes.
pub trait Spinner {
    fn new() -> Self;
    fn finish_and_error(&self, message: &str) -> !;
    fn finish_and_success(&self, message: &str) -> !;
}

impl Spinner for ProgressBar {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(80);
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        spinner
    }

    fn finish_and_error(&self, message: &str) -> ! {
        self.finish_and_clear();
        println!("{} {}", "x".red(), message);
        process::exit(1);
    }

    fn finish_and_success(&self, message: &str) -> ! {
        self.finish_and_clear();
        println!("{} {}", "✓".green(), message);
        process::exit(0);
    }
}
