use dropshot::{endpoint, HttpError, HttpResponseOk, Path, RequestContext};

#[dropshot::api_description]
pub trait ApiService {
    type Context;
}
