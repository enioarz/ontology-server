use clap::Arg;
use clap::ArgGroup;
use clap::ArgMatches;
use clap::builder::styling::AnsiColor;
use clap::{ArgAction, Command};
use dotenvy::dotenv;
use eyre::Result;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use hyper_ontology::config::{OntologyConfig, Settings};
use hyper_ontology::render_html::ArcOntologyRender;
use hyper_ontology::render_html::IRIMappedRenderHTML;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let cl = cli();
    let matches = cl.get_matches();
    match matches.subcommand() {
        Some(("build", _)) => {
            let settings = parser_app(Some(&matches))?;
            let mut or = ArcOntologyRender::new_with_settings(settings)?;

            let hm = or.render_all_declarations_html(None)?;
            fs::create_dir_all("public").unwrap_or(println!("Folder already exist"));
            for (k, v) in hm.iter() {
                match or.prefix_mapping.shrink_iri(k) {
                    Ok(i) => {
                        let iri_parts: Vec<String> = i
                            .to_string()
                            .split(":")
                            .map(|s: &str| s.to_string())
                            .collect();
                        let prefix_len = iri_parts.len();
                        if prefix_len == 1 {
                            fs::write(format!("public/{}.html", &iri_parts[0]), v).unwrap();
                        }
                    }
                    Err(_) => (),
                }
            }
            fs::write("public/index.html", or.render_metadata_html(None).unwrap()).unwrap();
            copy_dir_all("static", "public/static")?;
        }
        _ => {
            let mut help = cli();
            help.print_help()?;
        }
    };

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn parser_app(m: Option<&ArgMatches>) -> Result<Settings> {
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
    let pre = Figment::new().merge(env);

    let pre = if let Some(c) = m {
        if let Some(a) = c.get_one("Config").map(|m: &String| String::from(m)) {
            if Path::new(&a).exists() {
                pre.merge(Toml::file(&a))
            } else {
                pre
            }
        } else {
            pre
        }
    } else {
        pre
    };

    let settings: Settings = pre.extract()?;
    let fig: Figment = if let Some(matches) = m {
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
        let new_imports = match settings.import {
            Some(mut i) => match imports {
                Some(mut j) => {
                    i.append(&mut j);
                    Some(i)
                }
                None => Some(i),
            },
            None => imports,
        };
        let cli_settings = Settings {
            baseurl: if let Some(u) = matches.get_one("URL").map(|m: &String| String::from(m)) {
                Some(u)
            } else {
                settings.baseurl.clone()
            },
            ontology: OntologyConfig {
                iri: if let Some(i) = matches.get_one("IRI").map(|m: &String| String::from(m)) {
                    i
                } else {
                    settings.ontology.iri.clone()
                },
                source: if let Some(s) = matches.get_one("Source").map(|m: &String| String::from(m))
                {
                    Some(s)
                } else {
                    settings.ontology.source.clone()
                },
                suffix: if let Some(s) = matches.get_one("Suffix").map(|m: &String| String::from(m))
                {
                    Some(s)
                } else {
                    settings.ontology.suffix.clone()
                },
            },
            import: new_imports,
            templates: if let Some(t) = matches
                .get_one("Templates")
                .map(|m: &String| String::from(m))
            {
                Some(t)
            } else {
                settings.templates.clone()
            },
        };
        Figment::new().merge(Serialized::defaults(cli_settings))
    } else {
        Figment::new().merge(Serialized::defaults(settings))
    };

    Ok(fig.extract()?)
}

fn cli() -> Command {
    clap::command!()
        .name("hyppo")
        .styles(CLAP_STYLING)
        .bin_name("hyppo")
        .group(ArgGroup::new("general"))
        .next_help_heading("GENERAL")
        .arg(
            Arg::new("IRI")
                .action(ArgAction::Set)
                .help("IRI of the main ontology.")
                .group("general"),
        )
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
            Arg::new("Config")
                .long("config")
                .short('c')
                .action(ArgAction::Set)
                .help("Configuration directory.")
                .group("general"),
            Arg::new("Imported")
                .short('p')
                .long("import")
                .action(ArgAction::Append)
                .help("Imported ontologies to render. Expects 'prefix:iri' syntax.")
                .group("general"),
        ])
        .subcommand(
            clap::command!("build")
                .about("Build ontology static files.")
                .args([Arg::new("Render")
                    .long("render_imports")
                    .short('r')
                    .action(ArgAction::SetTrue)
                    .help("Render Imports.")
                    .group("general")]),
        )
        .subcommand_help_heading("Commands")
}

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(AnsiColor::Yellow.on_default())
    .usage(AnsiColor::Yellow.on_default())
    .literal(AnsiColor::Green.on_default())
    .placeholder(AnsiColor::Green.on_default());
