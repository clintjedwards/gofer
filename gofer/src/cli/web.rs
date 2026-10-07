use crate::cli::Cli;
use anyhow::{Context, Result};
use colored::Colorize;
use polyfmt::println;
use reqwest::Url;

impl Cli {
    /// Opens the web UI in a browser, already signed in with the CLI's token.
    ///
    /// The token itself never goes in the URL. Instead the API hands back a short lived, single use code, which rides
    /// in the URL fragment (never sent to the server) and which the page trades for the token over a POST.
    pub async fn web(
        &self,
        namespace_id: Option<String>,
        pipeline_id: Option<String>,
        run_id: Option<u64>,
        print_only: bool,
    ) -> Result<()> {
        let namespace = match namespace_id {
            Some(namespace) => namespace,
            None => self.conf.namespace.clone(),
        };

        let page = self.web_page(&namespace, pipeline_id, run_id).await?;

        let code = self.web_login_code().await?;

        let mut url = page.clone();
        if let Some(code) = &code {
            url.set_fragment(Some(&format!("login={code}")));
        }

        if !print_only {
            match open::that_detached(url.as_str()) {
                Ok(()) => {
                    // Only the page goes to the terminal so a sign in code doesn't linger in scrollback.
                    println!("Opening {} in your browser", page.as_str().cyan());
                    return Ok(());
                }
                Err(e) => println!("Could not open a browser ({e}); open this link instead:"),
            }
        }

        println!("{}", url.as_str().cyan());
        if code.is_some() {
            println!(
                "{}",
                "This link signs you in once and expires in 60 seconds. Don't share it.".dimmed()
            );
        }
        Ok(())
    }

    /// Gets a one time sign in code for the browser, or None when there's no usable token. Without one we still open
    /// the page; visitors can browse the default namespace anonymously and sign in from the page later.
    async fn web_login_code(&self) -> Result<Option<String>> {
        if self.conf.token.is_empty() {
            println!(
                "{}",
                "No token configured, so opening the web UI without signing in.".dimmed()
            );
            return Ok(None);
        }

        match self.client.create_web_login().await {
            Ok(login) => Ok(Some(login.into_inner().code)),
            Err(e)
                if matches!(
                    e.status(),
                    Some(reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN)
                ) =>
            {
                println!(
                    "{}",
                    "Your token wasn't accepted, so opening the web UI without signing in."
                        .dimmed()
                );
                Ok(None)
            }
            Err(e) => Err(e).context("Could not start a browser sign in with the Gofer api"),
        }
    }

    /// Picks the page to open: a run's details page, a pipeline's latest run, or the front page.
    async fn web_page(
        &self,
        namespace: &str,
        pipeline_id: Option<String>,
        run_id: Option<u64>,
    ) -> Result<Url> {
        let base = Url::parse(&self.conf.api_base_url).with_context(|| {
            format!("Could not parse api_base_url '{}'", self.conf.api_base_url)
        })?;

        let mut front_page = base.join("/")?;
        if namespace != "default" {
            front_page
                .query_pairs_mut()
                .append_pair("namespace", namespace);
        }

        let Some(pipeline_id) = pipeline_id else {
            return Ok(front_page);
        };

        let run_id = match run_id {
            Some(run_id) => run_id,
            None => {
                let latest = self
                    .client
                    .list_runs(namespace, &pipeline_id, Some(1), None, Some(true), None)
                    .await
                    .context("Could not retrieve the latest run from Gofer api")?
                    .into_inner()
                    .runs;

                match latest.first() {
                    Some(run) => run.run_id,
                    None => {
                        println!(
                            "Pipeline {} has no runs yet, so opening the front page",
                            pipeline_id.cyan()
                        );
                        return Ok(front_page);
                    }
                }
            }
        };

        let mut run_page = base.join("/run.html")?;
        run_page
            .query_pairs_mut()
            .append_pair("namespace", namespace)
            .append_pair("pipeline", &pipeline_id)
            .append_pair("run", &run_id.to_string());
        Ok(run_page)
    }
}
