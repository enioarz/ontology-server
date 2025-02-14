use serde::{Deserialize, Serialize};

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
