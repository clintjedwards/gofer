use crate::conf::LOCALHOST_CA;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone, econf::LoadEnv, PartialEq, Eq)]
pub struct Config {
    pub dev_mode: bool,
    pub namespace: Option<String>,
    pub server: String,
    pub tls_ca: Option<String>,
}

impl Config {
    pub fn inject_localhost_dev_certs(&mut self) {
        if !self.dev_mode || self.tls_ca.is_some() {
            return;
        }

        self.tls_ca = Some(LOCALHOST_CA.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conf::{Kind, LOCALHOST_CA};
    use config;

    #[test]
    /// Test that the TLS replacement for local builds works correctly.
    fn parse_default_config_with_default_tls() {
        let config_src_builder = config::Config::builder();

        let config = Kind::new_cli_config();

        let config_src = config_src_builder
            .add_source(config::File::from_str(
                config.default_config(),
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap();

        let mut parsed_config = config_src.try_deserialize::<Config>().unwrap();
        parsed_config.inject_localhost_dev_certs();

        let expected_config = Config {
            dev_mode: true,
            server: "https://127.0.0.1:8080".to_string(),
            namespace: None,
            tls_ca: Some(LOCALHOST_CA.to_string()),
        };

        assert_eq!(parsed_config, expected_config);
    }
}
