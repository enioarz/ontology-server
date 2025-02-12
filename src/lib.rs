pub mod models;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use curie::PrefixMapping;
use horned_owl::model::ForIRI;
use horned_owl::{
    model::{AnnotationSubject, AnnotationValue, Literal},
    visitor::immutable::Visit,
};
use lazy_static::lazy_static;
use models::{Kind, OntologyAnnotation, OntologyComponent, OntologyContent, OntologyMetadata};
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

impl<A: ForIRI> Default for OntologyContent<A> {
    fn default() -> Self {
        OntologyContent {
            metadata: OntologyMetadata::new(),
            hash_map: HashMap::new(),
            prefix_mapping: None,
        }
    }
}

impl<A: ForIRI> OntologyContent<A> {
    pub fn new_with_prefix_mapping(pm: PrefixMapping) -> Self {
        OntologyContent {
            metadata: OntologyMetadata::new(),
            hash_map: HashMap::new(),
            prefix_mapping: Some(pm),
        }
    }
}

impl OntologyMetadata {
    pub fn new() -> Self {
        OntologyMetadata {
            iri: None,
            version_iri: None,
            prev_iri: None,
            title: None,
            description: None,
            license: None,
            contributors: vec![],
            annotations: vec![],
        }
    }
}

impl<A: ForIRI> OntologyContent<A> {
    pub fn as_mut_hashmap(&mut self) -> &mut HashMap<A, OntologyComponent<A>> {
        &mut self.hash_map
    }

    pub fn into_hashmap(self) -> HashMap<A, OntologyComponent<A>> {
        self.hash_map
    }
}

impl<A: ForIRI> Visit<A> for OntologyContent<A> {
    fn visit_ontology_id(&mut self, oid: &horned_owl::model::OntologyID<A>) {
        match &oid.iri {
            Some(i) => {
                self.metadata.iri = Some(i.into());
            }
            None => (),
        }
        match &oid.viri {
            Some(vi) => {
                self.metadata.version_iri = Some(vi.into());
            }
            None => (),
        }
    }

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

    fn visit_ontology_annotation(&mut self, oa: &horned_owl::model::OntologyAnnotation<A>) {
        let ann = match unpack_annotation_value(&oa.0.av) {
            Some(vv) => {
                let iri_string = oa.0.ap.0.to_string();
                let label = match &self.prefix_mapping {
                    Some(pm) => match pm.shrink_iri(&iri_string) {
                        Ok(s) => s.into(),
                        Err(_) => iri_string.clone(),
                    },
                    None => iri_string.clone(),
                };
                let annotation = OntologyAnnotation {
                    iri: oa.0.ap.0.to_string(),
                    display: label,
                    value: vv,
                };
                Some(annotation)
            }
            None => None,
        };
        match ann {
            Some(aa) => match oa.0.ap.0.underlying().as_ref() {
                "http://purl.org/dc/elements/1.1/contributor" => {
                    self.metadata.contributors.push(aa)
                }
                "http://purl.org/dc/terms/title" => self.metadata.title = Some(aa.value),
                "http://purl.org/dc/terms/license" => self.metadata.license = Some(aa.value),
                "http://purl.org/dc/terms/description" => {
                    self.metadata.description = Some(aa.value)
                }
                _ => self.metadata.annotations.push(aa),
            },
            None => (),
        }
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
    pub fn render_html(&self) -> Result<String, tera::Error> {
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
            Some(e) => context.insert("example", &e),
            None => (),
        }
        match &self.kind {
            Kind::Class => context.insert("kind", "class"),
            Kind::ObjectProperty => context.insert("kind", "object-property"),
            Kind::AnnotationProperty => context.insert("kind", "annotation-property"),
            Kind::Undefined => context.insert("kind", "entity"),
        }
        context.insert("annotations", &self.annotations);

        match &self.kind {
            Kind::Class => TEMPLATES.render("entity.html", &context),
            Kind::ObjectProperty => TEMPLATES.render("entity.html", &context),
            Kind::AnnotationProperty => TEMPLATES.render("entity.html", &context),
            Kind::Undefined => TEMPLATES.render("entity.html", &context),
        }
    }
}

impl OntologyMetadata {
    pub fn render_html(&self) -> Result<String, tera::Error> {
        let mut context = Context::new();
        match &self.iri {
            Some(i) => context.insert("iri", i),
            None => (),
        }
        match &self.version_iri {
            Some(vi) => context.insert("version", vi),
            None => (),
        }
        match &self.title {
            Some(t) => context.insert("title", t),
            None => (),
        }
        match &self.description {
            Some(d) => context.insert("description", d),
            None => (),
        }
        match &self.license {
            Some(l) => context.insert("license", l),
            None => (),
        }
        context.insert("contributors", &self.contributors);
        context.insert("annotations", &self.annotations);
        TEMPLATES.render("ontology.html", &context)
    }
}
