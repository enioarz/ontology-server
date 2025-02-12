use curie::PrefixMapping;
use horned_owl::model::ForIRI;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct OntologyComponent<A: ForIRI> {
    pub iri: A,
    pub kind: Kind,
    pub label: Option<String>,
    pub definition: Option<String>,
    pub example: Option<String>,
    pub annotations: Vec<OntologyAnnotation>,
    pub parent: Option<A>,
    pub children: Vec<A>,
}

#[derive(Serialize, Deserialize)]
pub enum Kind {
    Class,
    ObjectProperty,
    AnnotationProperty,
    Undefined,
}

#[derive(Serialize, Deserialize)]
pub struct OntologyAnnotation {
    pub iri: String,
    pub display: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct OntologyMetadata {
    pub iri: Option<String>,
    pub version_iri: Option<String>,
    pub prev_iri: Option<String>,
    pub title: Option<String>,
    pub license: Option<String>,
    pub description: Option<String>,
    pub contributors: Vec<OntologyAnnotation>,
    pub annotations: Vec<OntologyAnnotation>,
}

#[derive(Serialize, Deserialize)]
pub struct OntologyContent<A: ForIRI> {
    pub metadata: OntologyMetadata,
    pub hash_map: HashMap<A, OntologyComponent<A>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub prefix_mapping: Option<PrefixMapping>,
}
