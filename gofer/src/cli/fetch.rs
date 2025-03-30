use crate::cli::Cli;
use anyhow::{bail, Result};

impl Cli {
    /// Lookup pipeline specific information. This is a quicker way to access information that commands like
    /// `pipeline list` or `run get` usually display.
    #[allow(clippy::too_many_arguments)]
    pub async fn fetch(
        &self,
        namespace_id: Option<String>,
        pipeline_id: Option<String>,
        run_id: Option<String>,
        task_id: Option<String>,
        limit: u64,
        offset: u64,
        no_reverse: bool,
    ) -> Result<()> {
        if let Some(task) = task_id {
            let run_id: u64 = match run_id.unwrap().parse() {
                Ok(run_id) => run_id,
                Err(e) => {
                    bail!("Could not parse into valid run integer; {:?}", e);
                }
            };

            match task.as_str() {
                "+" => {
                    return self
                        .task_list(namespace_id, &pipeline_id.unwrap(), run_id)
                        .await
                }
                _ => {
                    return self
                        .task_get(namespace_id, &pipeline_id.unwrap(), run_id, &task)
                        .await
                }
            }
        }

        if let Some(run) = run_id {
            match run.as_str() {
                "+" => {
                    return self
                        .run_list(
                            namespace_id,
                            &pipeline_id.unwrap(),
                            limit,
                            offset,
                            no_reverse,
                        )
                        .await;
                }
                _ => {
                    let run_id_int: u64 = match run.parse() {
                        Ok(run_id) => run_id,
                        Err(e) => {
                            bail!("Could not parse into valid run integer; {:?}", e);
                        }
                    };

                    return self
                        .run_get(namespace_id, &pipeline_id.unwrap(), run_id_int)
                        .await;
                }
            }
        }

        if pipeline_id.is_none() {
            return self.pipeline_list(namespace_id).await;
        }

        let pipeline = pipeline_id.unwrap();

        match pipeline.as_str() {
            "+" => return self.pipeline_list(namespace_id).await,
            _ => return self.pipeline_get(namespace_id, &pipeline).await,
        }
    }
}
