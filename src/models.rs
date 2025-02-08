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
pub struct ClassCollection<A: ForIRI> {
    pub hash_map: HashMap<A, OntologyComponent<A>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub prefix_mapping: Option<PrefixMapping>,
}
