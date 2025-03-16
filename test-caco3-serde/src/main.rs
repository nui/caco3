use figment::providers::{Format, Toml};
use figment::{Figment, Jail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(deserialize_with = "::caco3_serde::figment::pathbuf::deserialize_relative")]
    file: Option<PathBuf>,
}

const CONFIG_CONTENT: &str = indoc::indoc! {r#"
file = "../log/x.log"
"#};

fn main() {
    Jail::expect_with(|jail| {
        jail.create_dir("conf")?;
        jail.create_dir("log")?;
        jail.create_file("conf/config.toml", CONFIG_CONTENT)?;
        jail.create_file("log/x.log", "")?;
        let config: Config = Figment::from(Toml::file("conf/config.toml")).extract()?;
        dbg!(&config);
        dbg!(&config.file);
        dbg!(toml::to_string_pretty(&config).unwrap());
        Ok(())
    })
}
