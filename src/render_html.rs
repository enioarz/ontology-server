use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use curie::PrefixMapping;
use eyre::{Context, Result};
use horned_owl::model::{
    AnnotationProperty, AnnotationSubject, AnnotationValue, Class, ClassExpression,
    DeclareAnnotationProperty, DeclareClass, DeclareObjectProperty, Literal, ObjectProperty,
    ObjectPropertyExpression, ObjectPropertyRange, SubAnnotationPropertyOf, SubClassOf,
    SubObjectPropertyExpression, SubObjectPropertyOf,
};
use horned_owl::model::{Component, ComponentKind, ForIRI, IRI};
use horned_owl::ontology::indexed::ForIndex;
use horned_owl::ontology::iri_mapped::IRIMappedOntology;
use lazy_static::lazy_static;
use serde::Serialize;
use tera::Context as TeraContext;
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

#[derive(Serialize, Debug)]
struct EntityDisplay {
    iri: String,
    identifier: String,
    display: String,
}

#[derive(Serialize, Debug)]
struct AndDisplay(Vec<DisplayComp>);

#[derive(Serialize, Debug)]
struct SomeDisplay {
    first: Box<DisplayComp>,
    second: Box<DisplayComp>,
}

#[derive(Serialize, Debug)]
enum DisplayComp {
    Simple(EntityDisplay),
    And(AndDisplay),
    Some(SomeDisplay),
}

impl EntityDisplay {
    fn new(iri: String, identifier: String, display: String) -> Self {
        EntityDisplay {
            iri,
            identifier,
            display,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SideBar {
    classes: Vec<EntityDisplay>,
    annotation_props: Vec<EntityDisplay>,
    data_props: Vec<EntityDisplay>,
    object_props: Vec<EntityDisplay>,
}

impl Default for SideBar {
    fn default() -> Self {
        SideBar {
            classes: vec![],
            annotation_props: vec![],
            data_props: vec![],
            object_props: vec![],
        }
    }
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
        _: &HashMap<IRI<A>, String>,
    ) -> Result<String> {
        Err(eyre::Report::msg("Not implemented"))
    }

    fn render_all_declarations_html(
        &mut self,
        _: Option<&PrefixMapping>,
    ) -> Result<HashMap<IRI<A>, String>> {
        Err(eyre::Report::msg("Not implemented"))
    }
    fn render_metadata_html(&mut self, _: Option<PrefixMapping>) -> Result<String> {
        Err(eyre::Report::msg("Not implemented"))
    }

    fn get_iris_for_declaration(&mut self, _: ComponentKind) -> Vec<IRI<A>> {
        vec![]
    }

    fn get_label_hashmap(&mut self) -> HashMap<IRI<A>, String> {
        HashMap::new()
    }
    fn collect_entity_tree(&mut self, _: Option<&PrefixMapping>) -> Result<SideBar> {
        Err(eyre::Report::msg("Error when rendering tree"))
    }
}

impl<A: ForIRI, AA: ForIndex<A>> IRIMappedRenderHTML<A> for IRIMappedOntology<A, AA> {
    fn render_declaration_iri_html(
        &mut self,
        iri: &IRI<A>,
        pm: Option<&PrefixMapping>,
        lref: &HashMap<IRI<A>, String>,
    ) -> Result<String> {
        let mut context = TeraContext::new();
        let mut annotations: Vec<OntologyAnnotation> = vec![];
        let mut this_kind: Kind = Kind::Undefined;
        let mut super_entities: Vec<DisplayComp> = vec![];
        let mut sub_entities: Vec<DisplayComp> = vec![];
        for ann_cmp in self.components_for_iri(&iri) {
            let _ann = &ann_cmp.ann; // May add annotations later
            let cmp = &ann_cmp.component;
            match cmp {
                Component::DeclareClass(dc) => {
                    context.insert("iri", dc.0.0.as_ref());
                    this_kind = Kind::Class;
                    context.insert("kind", "klss")
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
                Component::SubClassOf(SubClassOf {
                    sup: ClassExpression::Class(spc),
                    sub: ClassExpression::Class(subc),
                }) => {
                    if &spc.0 == iri {
                        let child_display = build_entity_display(subc.0.clone(), pm, lref);
                        sub_entities.push(DisplayComp::Simple(child_display))
                    } else if &subc.0 == iri {
                        let parent_display = build_entity_display(spc.0.clone(), pm, lref);
                        super_entities.push(DisplayComp::Simple(parent_display));
                    }
                }
                Component::SubObjectPropertyOf(SubObjectPropertyOf {
                    sup: ObjectPropertyExpression::ObjectProperty(sup),
                    sub:
                        SubObjectPropertyExpression::ObjectPropertyExpression(
                            ObjectPropertyExpression::ObjectProperty(sub),
                        ),
                }) => {
                    if &sup.0 == iri {
                        let child_display = build_entity_display(sub.0.clone(), pm, lref);
                        sub_entities.push(DisplayComp::Simple(child_display))
                    } else if &sup.0 == iri {
                        let parent_display = build_entity_display(sub.0.clone(), pm, lref);
                        super_entities.push(DisplayComp::Simple(parent_display));
                    }
                }
                Component::SubDataPropertyOf(dp) => (),
                Component::EquivalentClasses(ec) => (),
                Component::EquivalentObjectProperties(eop) => (),
                Component::EquivalentDataProperties(edp) => (),
                Component::ObjectPropertyRange(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(ObjectProperty(ii)),
                    ce: ClassExpression::ObjectUnionOf(ce),
                }) => {
                    if ii == iri {
                        let mut union: Vec<DisplayComp> = vec![];
                        for c in ce.iter() {
                            match c {
                                ClassExpression::Class(class) => {
                                    let cls = build_entity_display(class.0.clone(), pm, lref);
                                    union.push(DisplayComp::Simple(cls))
                                }
                                _ => (),
                            }
                        }
                        context.insert("op_range", &DisplayComp::And(AndDisplay(union)));
                    }
                }
                Component::ObjectPropertyDomain(opd) => (),
                Component::DisjointClasses(djc) => (),
                Component::DisjointObjectProperties(djop) => (),
                Component::DisjointDataProperties(djdp) => (),
                Component::AnnotationPropertyRange(apr) => (),
                Component::AnnotationPropertyDomain(apd) => (),
                _ => (),
            }
        }
        if super_entities.len() > 0 {
            context.insert("super_classes", &super_entities);
        }
        if sub_entities.len() > 0 {
            context.insert("sub_classes", &sub_entities);
        }
        context.insert("annotations", &annotations);
        match this_kind {
            Kind::Class => TEMPLATES
                .render("entity.html", &context)
                .wrap_err("Could not render class page"),
            Kind::ObjectProperty => TEMPLATES
                .render("entity.html", &context)
                .wrap_err("Could not render object property page"),
            Kind::AnnotationProperty => TEMPLATES
                .render("entity.html", &context)
                .wrap_err("Could not render ann prop page"),
            Kind::Undefined => {
                Err(tera::Error::msg("Not implemented")).wrap_err("Unkown entity kind")
            }
        }
    }

    fn render_all_declarations_html(
        &mut self,
        pm: Option<&PrefixMapping>,
    ) -> Result<HashMap<IRI<A>, String>> {
        let mut declaration_hm: HashMap<IRI<A>, String> = HashMap::new();
        let label_reference = self.get_label_hashmap();
        for cl in self.get_iris_for_declaration(ComponentKind::DeclareClass) {
            let rendered_page = self.render_declaration_iri_html(&cl, pm, &label_reference)?;
            match declaration_hm.entry(cl) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for dp in self.get_iris_for_declaration(ComponentKind::DeclareDataProperty) {
            let rendered_page = self.render_declaration_iri_html(&dp, pm, &label_reference)?;
            match declaration_hm.entry(dp) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for op in self.get_iris_for_declaration(ComponentKind::DeclareObjectProperty) {
            let rendered_page = self.render_declaration_iri_html(&op, pm, &label_reference)?;
            match declaration_hm.entry(op) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for ap in self.get_iris_for_declaration(ComponentKind::DeclareAnnotationProperty) {
            let rendered_page = self.render_declaration_iri_html(&ap, pm, &label_reference)?;
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
    fn render_metadata_html(&mut self, pm: Option<PrefixMapping>) -> Result<String> {
        let mut context = TeraContext::default();
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
        let entity_tree = match self.collect_entity_tree(pm.as_ref()) {
            Ok(sb) => sb,
            Err(e) => {
                return Err(eyre::Report::msg(format!(
                    "Failed to collect entities for sidebar, context: {}",
                    e
                )));
            }
        };
        context.insert("sidebar", &entity_tree);
        context.insert("annotations", &annotations);
        context.insert("contributors", &contributors);
        Ok(TEMPLATES.render("ontology.html", &context)?)
    }

    fn collect_entity_tree(&mut self, pm: Option<&PrefixMapping>) -> Result<SideBar> {
        let labels: HashMap<IRI<A>, String> = self.get_label_hashmap();
        let mut side_bar = SideBar::default();
        for sco in self.component_for_kind(ComponentKind::DeclareClass) {
            match &sco.component {
                Component::DeclareClass(DeclareClass(Class(ii))) => {
                    let class_display = build_entity_display(ii.clone(), pm, &labels);
                    side_bar.classes.push(class_display)
                }
                _ => (),
            }
        }
        for sco in self.component_for_kind(ComponentKind::DeclareObjectProperty) {
            match &sco.component {
                Component::DeclareObjectProperty(DeclareObjectProperty(ObjectProperty(ii))) => {
                    let op_display = build_entity_display(ii.clone(), pm, &labels);
                    side_bar.object_props.push(op_display)
                }
                _ => (),
            }
        }
        for sco in self.component_for_kind(ComponentKind::DeclareAnnotationProperty) {
            match &sco.component {
                Component::DeclareAnnotationProperty(DeclareAnnotationProperty(
                    AnnotationProperty(ii),
                )) => {
                    let ap_display = build_entity_display(ii.clone(), pm, &labels);
                    side_bar.annotation_props.push(ap_display)
                }
                _ => (),
            }
        }

        for sco in self.component_for_kind(ComponentKind::DeclareDataProperty) {
            match &sco.component {
                Component::DeclareDataProperty(dp) => {
                    let class_iri = &dp.0.0;
                    let iri_string = class_iri.to_string();
                    let class_label = labels.get(class_iri).unwrap_or(&iri_string).clone();
                    let class_identifier = if let Some(pm) = pm {
                        match pm.shrink_iri(class_iri) {
                            Ok(r) => r.to_string(),
                            Err(e) => return Err(eyre::Report::msg(e)),
                        }
                    } else {
                        class_label.clone()
                    };
                    side_bar.data_props.push(EntityDisplay::new(
                        iri_string,
                        class_identifier,
                        class_label,
                    ))
                }
                _ => (),
            }
        }
        Ok(side_bar)
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

fn build_entity_display<A: ForIRI>(
    iri: IRI<A>,
    pm: Option<&PrefixMapping>,
    lref: &HashMap<IRI<A>, String>,
) -> EntityDisplay {
    let entity_id = if let Some(pm) = pm {
        match pm.shrink_iri(iri.as_ref()) {
            Ok(r) => {
                let mut s = String::from("");
                s.push_str(&r.to_string());
                s
            }
            Err(_) => iri.to_string(),
        }
    } else {
        iri.to_string()
    };
    let entity_label = match lref.get(&iri) {
        Some(l) => l.clone(),
        None => {
            if let Some(pm) = pm {
                match pm.shrink_iri(iri.as_ref()) {
                    Ok(r) => r.to_string(),
                    Err(_) => iri.to_string(),
                }
            } else {
                iri.to_string()
            }
        }
    };

    EntityDisplay::new(iri.to_string(), entity_id, entity_label)
}
