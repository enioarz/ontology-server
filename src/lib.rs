pub mod models;
use curie::PrefixMapping;
use horned_owl::model::{AnnotationValue, Literal};
use horned_owl::model::{Component, ComponentKind, ForIRI, IRI};
use horned_owl::ontology::indexed::ForIndex;
use horned_owl::ontology::iri_mapped::IRIMappedOntology;
use lazy_static::lazy_static;
use models::{Kind, OntologyAnnotation};
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

pub trait IRIMappedRenderHTML<A: ForIRI> {
    fn render_iri_html(
        &mut self,
        _: IRI<A>,
        _: Option<PrefixMapping>,
    ) -> Result<String, tera::Error> {
        Err(tera::Error::msg("Not implemented"))
    }

    fn render_metadata_html(&mut self, _: Option<PrefixMapping>) -> Result<String, tera::Error> {
        Err(tera::Error::msg("Not implemented"))
    }
}

impl<A: ForIRI, AA: ForIndex<A>> IRIMappedRenderHTML<A> for IRIMappedOntology<A, AA> {
    fn render_iri_html(
        &mut self,
        iri: IRI<A>,
        pm: Option<PrefixMapping>,
    ) -> Result<String, tera::Error> {
        let mut context = Context::new();
        let mut annotations: Vec<OntologyAnnotation> = vec![];
        let mut this_kind: Kind = Kind::Undefined;
        for ann_cmp in self.components_for_iri(&iri) {
            let _ann = &ann_cmp.ann; // May add annotations later
            let cmp = &ann_cmp.component;
            match cmp {
                Component::DeclareClass(dc) => {
                    context.insert("iri", dc.0.0.as_ref());
                    this_kind = Kind::Class;
                    context.insert("kind", "class")
                }
                Component::DeclareObjectProperty(op) => {
                    context.insert("iri", op.0.0.as_ref());
                    this_kind = Kind::ObjectProperty;
                    context.insert("kind", "object-property")
                }
                Component::DeclareAnnotationProperty(ap) => {
                    context.insert("iri", ap.0.0.as_ref());
                    this_kind = Kind::AnnotationProperty;
                    context.insert("kind", "annotation-property")
                }
                Component::AnnotationAssertion(aa) => match aa.ann.ap.0.as_ref() {
                    "http://www.w3.org/2000/01/rdf-schema#label" => context.insert(
                        "label",
                        get_annotation_value(&aa.ann.av).unwrap_or(iri.as_ref()),
                    ),
                    "http://www.w3.org/2004/02/skos/core#definition" => context.insert(
                        "definition",
                        get_annotation_value(&aa.ann.av).unwrap_or(iri.as_ref()),
                    ),
                    "http://www.w3.org/2004/02/skos/core#example" => context.insert(
                        "example",
                        get_annotation_value(&aa.ann.av).unwrap_or(iri.as_ref()),
                    ),
                    _ => match unpack_annotation_value(&aa.ann.av) {
                        Some(vv) => {
                            let label = match &pm {
                                Some(ppm) => match ppm.shrink_iri(aa.ann.ap.0.as_ref()) {
                                    Ok(s) => s.into(),
                                    Err(_) => aa.ann.ap.0.to_string(),
                                },
                                None => aa.ann.ap.0.to_string(),
                            };
                            let annotation = OntologyAnnotation {
                                iri: aa.ann.ap.0.to_string(),
                                display: label,
                                value: vv,
                            };
                            annotations.push(annotation);
                        }
                        None => (),
                    },
                },
                _ => (),
            }
        }
        context.insert("annotations", &annotations);
        match this_kind {
            Kind::Class => TEMPLATES.render("entity.html", &context),
            Kind::ObjectProperty => TEMPLATES.render("entity.html", &context),
            Kind::AnnotationProperty => TEMPLATES.render("entity.html", &context),
            Kind::Undefined => TEMPLATES.render("entity.html", &context),
        }
    }
    fn render_metadata_html(&mut self, pm: Option<PrefixMapping>) -> Result<String, tera::Error> {
        let mut context = Context::default();
        let mut contributors: Vec<OntologyAnnotation> = vec![];
        let mut annotations: Vec<OntologyAnnotation> = vec![];
        for oid in self.component_for_kind(ComponentKind::OntologyID) {
            match &oid.component {
                Component::OntologyID(oi) => {
                    match &oi.viri {
                        Some(i) => context.insert("version", i.as_ref()),
                        None => (),
                    }
                    match &oi.iri {
                        Some(i) => context.insert("iri", i.as_ref()),
                        None => (),
                    }
                }
                _ => (),
            }
        }
        for oann in self.component_for_kind(ComponentKind::OntologyAnnotation) {
            if let Component::OntologyAnnotation(oa) = &oann.component {
                let ann = match unpack_annotation_value(&oa.0.av) {
                    Some(vv) => {
                        let label = match &pm {
                            Some(pm) => match pm.shrink_iri(oa.0.ap.0.as_ref()) {
                                Ok(s) => s.into(),
                                Err(_) => oa.0.ap.0.to_string(),
                            },
                            None => oa.0.ap.0.to_string(),
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
                        "http://purl.org/dc/elements/1.1/contributor" => contributors.push(aa),
                        "http://purl.org/dc/terms/title" => context.insert("title", &aa.value),
                        "http://purl.org/dc/terms/license" => context.insert("license", &aa.value),
                        "http://purl.org/dc/terms/description" => {
                            context.insert("description", &aa.value)
                        }
                        _ => annotations.push(aa),
                    },
                    None => (),
                }
            } else {
            }
        }
        context.insert("annotations", &annotations);
        context.insert("contributors", &contributors);
        TEMPLATES.render("ontology.html", &context)
    }
}

fn get_annotation_value<A: ForIRI>(av: &AnnotationValue<A>) -> Option<&str> {
    match &av {
        AnnotationValue::AnonymousIndividual(_) => None,
        AnnotationValue::Literal(l) => match l {
            Literal::Simple { literal } => Some(literal),
            Literal::Language { literal, lang: _ } => Some(literal),
            Literal::Datatype {
                literal,
                datatype_iri: _,
            } => Some(literal),
        },
        AnnotationValue::IRI(ii) => Some(ii),
    }
}
