use serde::Deserialize;

#[derive(Deserialize)]
pub enum Kind {
    Class,
    ObjectProperty,
    AnnotationProperty,
    Undefined,
}

#[derive(Deserialize)]
pub struct OntologyAnnotation {
    pub iri: String,
    pub display: String,
    pub value: String,
}
