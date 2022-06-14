use crate::api::{Api, BUILD_COMMIT, BUILD_SEMVER};
use gofer_proto::GetSystemInfoResponse;
use tonic::{Response, Status};

impl Api {
    pub fn get_system_info_handler(&self) -> Result<Response<GetSystemInfoResponse>, Status> {
        Ok(Response::new(GetSystemInfoResponse {
            commit: BUILD_COMMIT.to_string(),
            dev_mode_enabled: self.conf.general.dev_mode,
            semver: BUILD_SEMVER.to_string(),
        }))
    }
}
