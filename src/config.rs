use clap::ArgMatches;
use clap::{ArgAction, Command, arg, command, value_parser};

use horned_owl::io::ParserConfiguration;
use horned_owl::io::RDFParserConfiguration;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[allow(unused)]
pub struct OntologyConfig {
    pub iri: String,
    pub source: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Deserialize, Debug, Serialize)]
#[allow(unused)]
pub struct Settings {
    pub ontology: OntologyConfig,
    pub baseurl: Option<String>,
    pub import: Option<Vec<OntologyConfig>>,
    pub templates: Option<String>,
}

pub fn parser_config(matches: &ArgMatches) -> ParserConfiguration {
    ParserConfiguration {
        rdf: RDFParserConfiguration {
            lax: !matches.get_one::<bool>("strict").unwrap_or(&false),
        },
        ..Default::default()
    }
}
