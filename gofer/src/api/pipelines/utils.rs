use crate::api::Api;
use std::collections::HashMap;

impl Api {
    /// Returns true if there are more runs in progress than the parallelism limit
    /// of a pipeline allows.
    /// If there was an error getting the current number of runs, we fail closed as the
    /// functionality of failing a parallelism_limit is usually retrying until it succeeds.
    pub async fn parallelism_limit_exceeded(
        &self,
        namespace_id: &str,
        pipeline_id: &str,
        limit: u64,
    ) -> bool {
        let mut limit = limit;

        if limit == 0 && self.conf.general.run_parallelism_limit == 0 {
            return false;
        }

        if limit > self.conf.general.run_parallelism_limit {
            limit = self.conf.general.run_parallelism_limit
        }

        let runs = match self
            .storage
            .list_runs(0, 0, namespace_id, pipeline_id)
            .await
        {
            Ok(runs) => runs,
            Err(_) => return true,
        };

        let mut runs_in_progress = 0;
        for run in runs {
            if run.state != gofer_models::RunState::Complete {
                runs_in_progress += 1;
            }
        }

        if runs_in_progress >= limit {
            return true;
        }

        false
    }
}

pub fn map_to_variables(
    map: HashMap<String, String>,
    owner: gofer_models::VariableOwner,
    sensitivity: gofer_models::VariableSensitivity,
) -> Vec<gofer_models::Variable> {
    let mut variables = vec![];

    for (key, value) in map {
        variables.push(gofer_models::Variable {
            key,
            value,
            owner: owner.clone(),
            sensitivity: sensitivity.clone(),
        })
    }

    variables
}
