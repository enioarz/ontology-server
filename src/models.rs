use serde::Serialize;

#[derive(Serialize)]
pub enum Kind {
    Class,
    ObjectProperty,
    AnnotationProperty,
    Undefined,
}

#[derive(Serialize)]
pub struct OntologyAnnotation {
    pub iri: String,
    pub display: String,
    pub value: String,
}
