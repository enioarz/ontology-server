use clap::Arg;
use clap::ArgGroup;
use clap::{ArgAction, Command};
use dotenvy::dotenv;
use eyre::Result;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use hyper_ontology::config::{OntologyConfig, Settings};
fn main() -> Result<()> {
    parser_app()?;
    Ok(())
}

pub fn parser_app() -> Result<Settings> {
    dotenv()?;
    let env = Env::raw()
        .map(|k| match k.starts_with("HYPPO_") {
            true => k["HYPPO_".len()..].into(),
            false => k.into(),
        })
        .map(|k| match k.starts_with("ONTOLOGY_") {
            true => k.to_string().replace("_", ".").into(),
            false => k.into(),
        });
    let settings: Settings = Figment::new()
        .merge(env)
        .merge(Toml::file("configs/example.toml"))
        .extract()?;
    let matches = cli().get_matches();
    let imports: Option<Vec<OntologyConfig>> = matches.get_many("Imported").map(|m| {
        m.map(|i: &String| {
            let content: Vec<String> = i.split(":").map(|x| String::from(x)).collect();
            content
        })
        .filter(|c| c.len() == 2)
        .map(|c| OntologyConfig {
            iri: c[1].clone(),
            source: None,
            suffix: Some(c[0].clone()),
        })
        .collect()
    });
    let cli_settings = Settings {
        baseurl: matches.get_one("URL").map(|m: &String| String::from(m)),
        ontology: OntologyConfig {
            iri: if let Some(i) = matches.get_one("IRI").map(|m: &String| String::from(m)) {
                i
            } else {
                settings.ontology.iri.clone()
            },
            source: matches.get_one("Source").map(|m: &String| String::from(m)),
            suffix: matches.get_one("Suffix").map(|m: &String| String::from(m)),
        },
        import: imports,
        templates: matches
            .get_one("Templates")
            .map(|m: &String| String::from(m)),
    };

    Ok(Figment::new()
        .merge(Serialized::defaults(settings))
        .merge(Serialized::defaults(cli_settings))
        .extract()?)
}

fn cli() -> Command {
    clap::command!()
        .name("hyppo")
        .bin_name("hyppo")
        .group(ArgGroup::new("general"))
        .next_help_heading("GENERAL")
        .arg(Arg::new("IRI")
            .required(true)
            .action(ArgAction::Set)
            .help("IRI of the main ontology.")
            .group("general"))
        .args([
            Arg::new("Source")
                .long("source")
                .action(ArgAction::Set)
                .help("Source of the ontology file (currently only supports local files)")
                .group("general"),
            Arg::new("Suffix")
                .long("suffix")
                .action(ArgAction::Set)
                .help("Suffix of the ontology")
                .group("general"),
            Arg::new("URL")
                .long("url")
                .short('u')
                .action(ArgAction::Set)
                .help("Base URL of the hosted service.")
                .group("general"),
            Arg::new("Templates")
                .long("templates")
                .short('t')
                .action(ArgAction::Set)
                .help("Tera templates directory.")
                .default_value("templates")
                .group("general"),
            Arg::new("Imported")
                .short('p')
                .long("import")
                .action(ArgAction::Append)
                .help("Imported ontologies to render. Expects 'prefix:iri' syntax.")
                .group("general")
        ])
        .subcommand(
            clap::command!("build")
                .args([
                    Arg::new("empty")
                        .long("empty")
                        .action(ArgAction::Append)
                        .help("File is empty and is either a regular file or a directory")
                        .group("build"),
                    Arg::new("name")
                        .long("name")
                        .action(ArgAction::Append)
                        .help("Base of file name (the path with the leading directories removed) matches shell pattern pattern")
                        .group("build")
                ])
        )
}
