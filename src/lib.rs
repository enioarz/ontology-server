pub mod models;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use horned_owl::model::ForIRI;
use horned_owl::{
    model::{AnnotationSubject, AnnotationValue, Literal},
    visitor::immutable::Visit,
};
use lazy_static::lazy_static;
use models::{ClassCollection, Kind, OntologyAnnotation, OntologyComponent};
use tera::Context;
use tera::Tera;
lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera
    };
}

impl<A: ForIRI> OntologyComponent<A> {
    fn new(iri: A, kind: Kind) -> Self {
        OntologyComponent {
            iri,
            definition: None,
            kind,
            label: None,
            example: None,
            annotations: vec![],
            parent: None,
            children: vec![],
        }
    }
}

impl<A: ForIRI> Default for ClassCollection<A> {
    fn default() -> Self {
        ClassCollection {
            hash_map: HashMap::new(),
            prefix_mapping: None,
        }
    }
}

impl<A: ForIRI> ClassCollection<A> {
    pub fn as_mut_hashmap(&mut self) -> &mut HashMap<A, OntologyComponent<A>> {
        &mut self.hash_map
    }

    pub fn into_hashmap(self) -> HashMap<A, OntologyComponent<A>> {
        self.hash_map
    }
}

impl<A: ForIRI> Visit<A> for ClassCollection<A> {
    fn visit_class(&mut self, c: &horned_owl::model::Class<A>) {
        match self.hash_map.entry(c.0.underlying()) {
            Entry::Occupied(o) => {
                let oc = o.into_mut();
                oc.iri = c.0.underlying();
                oc.kind = Kind::Class;
            }
            Entry::Vacant(v) => {
                v.insert(OntologyComponent::new(c.0.underlying(), Kind::Class));
            }
        }
    }

    fn visit_annotation_property(&mut self, ap: &horned_owl::model::AnnotationProperty<A>) {
        match self.hash_map.entry(ap.0.underlying()) {
            Entry::Occupied(o) => {
                let oc = o.into_mut();
                oc.iri = ap.0.underlying();
                oc.kind = Kind::AnnotationProperty;
            }
            Entry::Vacant(v) => {
                v.insert(OntologyComponent::new(
                    ap.0.underlying(),
                    Kind::AnnotationProperty,
                ));
            }
        }
    }

    fn visit_object_property(&mut self, op: &horned_owl::model::ObjectProperty<A>) {
        match self.hash_map.entry(op.0.underlying()) {
            Entry::Occupied(o) => {
                let oc = o.into_mut();
                oc.iri = op.0.underlying();
                oc.kind = Kind::ObjectProperty;
            }
            Entry::Vacant(v) => {
                v.insert(OntologyComponent::new(
                    op.0.underlying(),
                    Kind::ObjectProperty,
                ));
            }
        }
    }

    fn visit_annotation_assertion(&mut self, aa: &horned_owl::model::AnnotationAssertion<A>) {
        match &aa.subject {
            AnnotationSubject::IRI(i) => {
                let ontology_class = match self.hash_map.entry(i.underlying()) {
                    Entry::Occupied(o) => {
                        let oc = o.into_mut();
                        oc
                    }
                    Entry::Vacant(v) => {
                        v.insert(OntologyComponent::new(i.underlying(), Kind::Undefined))
                    }
                };
                match aa.ann.ap.0.underlying().as_ref() {
                    "http://www.w3.org/2000/01/rdf-schema#label" => {
                        ontology_class.label = unpack_annotation_value(&aa.ann.av)
                    }
                    "http://www.w3.org/2004/02/skos/core#definition" => {
                        ontology_class.definition = unpack_annotation_value(&aa.ann.av)
                    }
                    "http://www.w3.org/2004/02/skos/core#example" => {
                        ontology_class.example = unpack_annotation_value(&aa.ann.av)
                    }
                    _ => match unpack_annotation_value(&aa.ann.av) {
                        Some(vv) => {
                            let iri_string = aa.ann.ap.0.to_string();
                            let label = match &self.prefix_mapping {
                                Some(pm) => match pm.shrink_iri(&iri_string) {
                                    Ok(s) => s.into(),
                                    Err(_) => iri_string.clone(),
                                },
                                None => iri_string.clone(),
                            };
                            let annotation = OntologyAnnotation {
                                iri: aa.ann.ap.0.to_string(),
                                display: label,
                                value: vv,
                            };
                            ontology_class.annotations.push(annotation);
                        }
                        None => (),
                    },
                };
            }
            AnnotationSubject::AnonymousIndividual(_) => (),
        };
    }
}

fn unpack_annotation_value<A: ForIRI>(av: &AnnotationValue<A>) -> Option<String> {
    match &av {
        AnnotationValue::AnonymousIndividual(_) => None,
        AnnotationValue::Literal(l) => match l {
            Literal::Simple { literal } => Some(literal.clone()),
            Literal::Language { literal, lang: _ } => Some(literal.clone()),
            Literal::Datatype {
                literal,
                datatype_iri: _,
            } => Some(literal.clone()),
        },
        AnnotationValue::IRI(ii) => Some(ii.to_string()),
    }
}

impl<A: ForIRI> OntologyComponent<A> {
    pub fn render(&self) -> Result<String, tera::Error> {
        let mut context = Context::new();
        match &self.label {
            Some(l) => context.insert("label", &l),
            None => context.insert("label", &self.iri.as_ref()),
        }
        context.insert("iri", &self.iri.as_ref());
        match &self.definition {
            Some(d) => context.insert("definition", &d),
            None => (),
        }
        match &self.example {
            Some(e) => context.insert("definition", &e),
            None => (),
        }

        context.insert("annotations", &self.annotations);

        match &self.kind {
            Kind::Class => TEMPLATES.render("base.html", &context),
            Kind::ObjectProperty => TEMPLATES.render("base.html", &context),
            Kind::AnnotationProperty => TEMPLATES.render("base.html", &context),
            Kind::Undefined => TEMPLATES.render("base.html", &context),
        }
    }
}
