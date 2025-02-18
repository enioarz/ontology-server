use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use curie::PrefixMapping;
use horned_owl::model::{AnnotationSubject, AnnotationValue, ClassExpression, Literal};
use horned_owl::model::{Component, ComponentKind, ForIRI, IRI};
use horned_owl::ontology::indexed::ForIndex;
use horned_owl::ontology::iri_mapped::IRIMappedOntology;
use lazy_static::lazy_static;
use serde::Serialize;
use tera::Context;
use tera::Tera;

#[derive(Debug, Clone)]
pub struct RenderError(String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize)]
enum Kind {
    Class,
    ObjectProperty,
    AnnotationProperty,
    Undefined,
}

#[derive(Serialize)]
struct OntologyAnnotation {
    iri: String,
    display: String,
    value: String,
}

struct EntityRepr {
    iri: String,
    display: String,
}

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
    fn render_declaration_iri_html(
        &mut self,
        _: &IRI<A>,
        _: Option<&PrefixMapping>,
    ) -> Result<String, tera::Error> {
        Err(tera::Error::msg("Not implemented"))
    }

    fn render_all_declarations_html(
        &mut self,
        _: Option<&PrefixMapping>,
    ) -> Result<HashMap<IRI<A>, String>, tera::Error> {
        Err(tera::Error::msg("Not implemented"))
    }
    fn render_metadata_html(&mut self, _: Option<PrefixMapping>) -> Result<String, tera::Error> {
        Err(tera::Error::msg("Not implemented"))
    }

    fn get_iris_for_declaration(&mut self, _: ComponentKind) -> Vec<IRI<A>> {
        vec![]
    }

    fn get_label_hashmap(&mut self) -> HashMap<IRI<A>, String> {
        HashMap::new()
    }
    fn render_tree_html(&mut self, _: Option<IRI<A>>) -> Result<String, RenderError> {
        Err(RenderError("Error when rendering tree".into()))
    }
}

impl<A: ForIRI, AA: ForIndex<A>> IRIMappedRenderHTML<A> for IRIMappedOntology<A, AA> {
    fn render_declaration_iri_html(
        &mut self,
        iri: &IRI<A>,
        pm: Option<&PrefixMapping>,
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
            Kind::Undefined => Err(tera::Error::msg("Not implemented")),
        }
    }

    fn render_all_declarations_html(
        &mut self,
        pm: Option<&PrefixMapping>,
    ) -> Result<HashMap<IRI<A>, String>, tera::Error> {
        let mut declaration_hm: HashMap<IRI<A>, String> = HashMap::new();
        for cl in self.get_iris_for_declaration(ComponentKind::DeclareClass) {
            let rendered_page = self.render_declaration_iri_html(&cl, pm)?;
            match declaration_hm.entry(cl) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for dp in self.get_iris_for_declaration(ComponentKind::DeclareDataProperty) {
            let rendered_page = self.render_declaration_iri_html(&dp, pm)?;
            match declaration_hm.entry(dp) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for op in self.get_iris_for_declaration(ComponentKind::DeclareObjectProperty) {
            let rendered_page = self.render_declaration_iri_html(&op, pm)?;
            match declaration_hm.entry(op) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for ap in self.get_iris_for_declaration(ComponentKind::DeclareAnnotationProperty) {
            let rendered_page = self.render_declaration_iri_html(&ap, pm)?;
            match declaration_hm.entry(ap) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        Ok(declaration_hm)
    }

    fn get_iris_for_declaration(&mut self, component_kind: ComponentKind) -> Vec<IRI<A>> {
        self.component_for_kind(component_kind)
            .map(|dc| match &dc.component {
                Component::DeclareClass(dc) => Some(dc.0.0.clone()),
                Component::DeclareDataProperty(ddp) => Some(ddp.0.0.clone()),
                Component::DeclareObjectProperty(dop) => Some(dop.0.0.clone()),
                Component::DeclareAnnotationProperty(dap) => Some(dap.0.0.clone()),
                _ => None,
            })
            .filter(|x| match x {
                Some(_) => true,
                None => false,
            })
            .map(|y| y.unwrap())
            .collect()
    }

    fn get_label_hashmap(&mut self) -> HashMap<IRI<A>, String> {
        let mut label_map: HashMap<IRI<A>, String> = HashMap::new();

        for aa in self.component_for_kind(ComponentKind::AnnotationAssertion) {
            match &aa.component {
                Component::AnnotationAssertion(aas) => match &aas.subject {
                    AnnotationSubject::IRI(iri) => match aas.ann.ap.0.as_ref() {
                        "http://www.w3.org/2000/01/rdf-schema#label" => match &aas.ann.av {
                            AnnotationValue::Literal(literal) => {
                                label_map.insert(iri.clone(), literal.literal().clone());
                            }
                            _ => (),
                        },
                        _ => (),
                    },
                    AnnotationSubject::AnonymousIndividual(_) => (),
                },
                _ => (),
            }
        }

        label_map
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

    fn render_tree_html(&mut self, upper_term: Option<IRI<A>>) -> Result<String, RenderError> {
        for sco in self.component_for_kind(ComponentKind::SubClassOf) {
            match &sco.component {
                Component::SubClassOf(sc) => {
                    if let ClassExpression::Class(c) = &sc.sup {
                        if let ClassExpression::Class(d) = &sc.sub {}
                    }
                }
                _ => (),
            }
        }
        Err(RenderError("Not implemented".into()))
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
