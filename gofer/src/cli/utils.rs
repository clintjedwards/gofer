use anyhow::anyhow;
use std::collections::HashMap;
use std::process;
use std::str::FromStr;
use tonic::transport::{Certificate, ClientTlsConfig, Uri};

pub fn printerr_and_finish(message: &str) -> ! {
    eprintln!("Command failed; {}", message);
    process::exit(1);
}

pub fn parse_variables(vars: Vec<String>) -> HashMap<String, String> {
    let v: HashMap<String, String> = vars
        .into_iter()
        .map(|var| {
            let split_var = var.split_once('=');
            match split_var {
                None => {
                    eprintln!(
                        "Variable parsing error for var '{}'; must be in form my_key=my_var",
                        var
                    );
                    process::exit(1);
                }
                Some((key, value)) => (key.to_string(), value.to_string()),
            }
        })
        .collect();

    v
}

/// Returns a valid TLS configuration for GRPC connections. Most of this is only required to make
/// self-signed cert usage easier. Rustls wont allow IP addresses in the url field and wont allow
/// you to skip client-side issuer verification. So if the user enters 127.0.0.1 we replace
/// it with the domain "localhost" and if the user supplies us with a root cert that trusts the
/// localhost certs we add it to the root certificate trust store.
pub fn get_tls_config(url: &str, ca_cert: Option<String>) -> anyhow::Result<ClientTlsConfig> {
    let uri = Uri::from_str(url)?;
    let mut domain_name = uri
        .host()
        .ok_or_else(|| anyhow!("could not get domain name from uri: {:?}", uri))?;
    if domain_name.eq("127.0.0.1") {
        domain_name = "localhost"
    }

    let mut tls_config = ClientTlsConfig::new().domain_name(domain_name);

    if let Some(ca_cert) = ca_cert {
        tls_config = tls_config.ca_certificate(Certificate::from_pem(ca_cert));
    }

    Ok(tls_config)
}
