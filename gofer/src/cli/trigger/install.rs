use super::super::CliHarness;
use crate::cli::{parse_variables, printerr_and_finish, Spinner};
use colored::Colorize;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use gofer_proto::gofer_client::GoferClient;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::io::{stdin, stdout, Write};
use tonic::transport::Channel;

fn to_title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl CliHarness {
    async fn run_trigger_installer(
        &self,
        client: &mut GoferClient<Channel>,
        name: &str,
        image: &str,
        user: Option<String>,
        pass: Option<String>,
    ) -> HashMap<String, String> {
        let spinner: ProgressBar = Spinner::new();
        spinner.set_message("Fetching trigger instructions");

        let request = tonic::Request::new(gofer_proto::GetTriggerInstallInstructionsRequest {
            image: image.to_string(),
            user: user.clone().unwrap_or_default(),
            pass: pass.clone().unwrap_or_default(),
        });
        let response = client
            .get_trigger_install_instructions(request)
            .await
            .unwrap_or_else(|e| {
                spinner.finish_and_error(&format!(
                    "Could not get trigger install instructions; {}",
                    e
                ));
            })
            .into_inner();

        let instructions_str = response.instructions.trim();

        spinner.println_success("Fetched trigger instructions");
        spinner.set_message("Parsing trigger instructions");

        let instructions: gofer_sdk::trigger::InstallInstructions =
            serde_json::from_str(instructions_str).unwrap_or_else(|e| {
                spinner.finish_and_error(&format!(
                    "Could not parse trigger instructions from json; {}",
                    e
                ));
            });

        spinner.println_success("Parsed trigger instructions");
        spinner.finish_and_clear();

        execute!(stdout(), EnterAlternateScreen).unwrap();

        let mut config_map: HashMap<String, String> = HashMap::new();

        println!(
            "{}\n",
            format!(":: {} Trigger Setup", to_title_case(name)).cyan()
        );
        for instruction in instructions.instructions {
            match instruction {
                gofer_sdk::trigger::InstallInstruction::Message { text } => {
                    println!("{}", text.trim());
                }
                gofer_sdk::trigger::InstallInstruction::Query { text, config_key } => {
                    print!("> {}: ", text.trim());
                    stdout().flush().unwrap();

                    let mut input_string = String::new();
                    stdin().read_line(&mut input_string).unwrap_or_else(|e| {
                        printerr_and_finish(&format!("Could not parse input; {:?}", e));
                    });

                    config_map.insert(config_key, input_string.trim().to_string());
                }
            }
        }

        print!(
            "Install trigger '{}' with above settings? [Y/n]: ",
            to_title_case(name)
        );
        stdout().flush().unwrap();

        let mut input_string = String::new();
        stdin().read_line(&mut input_string).unwrap_or_else(|e| {
            printerr_and_finish(&format!("Could not parse input; {:?}", e));
        });

        let input_string = input_string.trim().to_lowercase();

        if input_string != "y" {
            printerr_and_finish("Abandoned installation process");
        }

        execute!(stdout(), LeaveAlternateScreen).unwrap();

        config_map
    }

    pub async fn trigger_install(
        &self,
        name: &str,
        image: &str,
        user: Option<String>,
        pass: Option<String>,
        installer: bool,
        variables: Vec<String>,
    ) {
        let mut client = match self.connect().await {
            Ok(client) => client,
            Err(e) => {
                printerr_and_finish(&format!("Could not connect to Gofer server; {}", e));
            }
        };

        let mut config_map = parse_variables(variables);

        if installer {
            config_map = self
                .run_trigger_installer(&mut client, name, image, user.clone(), pass.clone())
                .await;
        }

        let spinner: ProgressBar = Spinner::new();

        if installer {
            spinner.println_success("Collected trigger configuration");
        }

        spinner.set_message("Installing trigger");

        let request = tonic::Request::new(gofer_proto::InstallTriggerRequest {
            name: name.to_string(),
            image: image.to_string(),
            user: user.unwrap_or_default(),
            pass: pass.unwrap_or_default(),
            variables: config_map,
        });
        client
            .install_trigger(request)
            .await
            .unwrap_or_else(|e| {
                spinner.finish_and_error(&format!("Could not install trigger; {:?}", e));
            })
            .into_inner();

        spinner.finish_and_success("Installed trigger")
    }
}
