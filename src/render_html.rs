use curie::PrefixMapping;
use eyre::{Context, Result};
use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::{
    AnnotatedComponent, AnnotationProperty, AnnotationSubject, AnnotationValue, ArcStr, Build,
    Class, ClassAssertion, ClassExpression, DeclareAnnotationProperty, DeclareClass,
    DeclareNamedIndividual, DeclareObjectProperty, EquivalentClasses, Individual,
    InverseObjectProperties, Literal, NamedIndividual, ObjectProperty, ObjectPropertyDomain,
    ObjectPropertyExpression, ObjectPropertyRange, RcStr, SubClassOf, SubObjectPropertyExpression,
    SubObjectPropertyOf,
};
use horned_owl::model::{Component, ComponentKind, ForIRI, IRI};
use horned_owl::ontology::indexed::ForIndex;
use horned_owl::ontology::iri_mapped::IRIMappedOntology;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::rc::Rc;
use std::sync::Arc;
use tera::Context as TeraContext;
use tera::Tera;

use crate::config::Settings;

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
    NamedIndividual,
    Undefined,
}

#[derive(Serialize)]
struct OntologyAnnotation {
    iri: String,
    display: String,
    value: String,
}

#[derive(Serialize, Debug)]
pub struct EntityDisplay {
    pub iri: String,
    pub identifier: String,
    pub display: String,
}

#[derive(Serialize, Debug)]
pub struct GroupDisplay(Vec<DisplayComp>);

#[derive(Serialize, Debug)]
pub struct RelDisplay {
    rel: Box<DisplayComp>,
    ce: Box<DisplayComp>,
}

#[derive(Serialize, Debug)]
pub struct DPDisplay {
    dp: Box<DisplayComp>,
    value: String,
}

#[derive(Serialize, Debug)]
pub enum DisplayComp {
    Simple(EntityDisplay),
    And(GroupDisplay),
    Or(GroupDisplay),
    Some(RelDisplay),
    Value(RelDisplay),
    All(RelDisplay),
    Not(Box<DisplayComp>),
    Data(DPDisplay),
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
    named_individuals: Vec<EntityDisplay>,
    annotation_props: Vec<EntityDisplay>,
    data_props: Vec<EntityDisplay>,
    object_props: Vec<EntityDisplay>,
}

impl Default for SideBar {
    fn default() -> Self {
        SideBar {
            classes: vec![],
            named_individuals: vec![],
            annotation_props: vec![],
            data_props: vec![],
            object_props: vec![],
        }
    }
}

pub trait IRIMappedRenderHTML<A: ForIRI> {
    fn render_declaration_iri_html(&mut self, _: &IRI<A>, _: Option<&str>) -> Result<String> {
        Err(eyre::Report::msg("Not implemented"))
    }

    fn render_all_declarations_html(&mut self, _: Option<&str>) -> Result<HashMap<IRI<A>, String>> {
        Err(eyre::Report::msg("Not implemented"))
    }
    fn render_metadata_html(&mut self, _: Option<&str>) -> Result<String> {
        Err(eyre::Report::msg("Not implemented"))
    }

    fn get_iris_for_declaration(&mut self, _: ComponentKind) -> Vec<IRI<A>> {
        vec![]
    }
    fn collect_entity_tree(&mut self, _: Option<&str>) -> Result<SideBar> {
        Err(eyre::Report::msg("Error when rendering tree"))
    }

    fn build_entity_display(&self, _: IRI<A>, _: &str) -> EntityDisplay {
        todo!("build_entity_display has to be implemented")
    }

    fn unpack_class_expression(&self, _: ClassExpression<A>, _: Option<&str>) -> DisplayComp {
        todo!("unpack_class_expression is not implemented",)
    }

    fn unpack_object_property_expression(
        &self,
        _: ObjectPropertyExpression<A>,
        _: Option<&str>,
    ) -> DisplayComp {
        todo!()
    }
}

pub struct OntologyRender<A: ForIRI, AA: ForIndex<A>> {
    pub ontology: IRIMappedOntology<A, AA>,
    pub prefix_mapping: PrefixMapping,
    pub label_map: HashMap<IRI<A>, String>,
    pub settings: Settings,
    pub templates: Tera,
}

pub type RcOntologyRender = OntologyRender<RcStr, Rc<AnnotatedComponent<RcStr>>>;
pub type ArcOntologyRender = OntologyRender<ArcStr, Arc<AnnotatedComponent<ArcStr>>>;

impl<A: ForIRI, AA: ForIndex<A>> IRIMappedRenderHTML<A> for OntologyRender<A, AA> {
    fn render_declaration_iri_html(&mut self, iri: &IRI<A>, base: Option<&str>) -> Result<String> {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri,
        };
        let mut context = TeraContext::new();
        let mut annotations: Vec<OntologyAnnotation> = vec![];
        let mut this_kind: Kind = Kind::Undefined;
        let mut super_entities: Vec<DisplayComp> = vec![];
        let mut inverse_ops: Vec<DisplayComp> = vec![];
        let mut sub_entities: Vec<DisplayComp> = vec![];
        let mut equivalent_classes: Vec<DisplayComp> = vec![];
        let mut class_assertions: Vec<DisplayComp> = vec![];
        let anns: Vec<AnnotatedComponent<A>> = self
            .ontology
            .components_for_iri(&iri)
            .map(|x| x.clone())
            .collect();
        for ann_cmp in anns {
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
                Component::DeclareNamedIndividual(ni) => {
                    context.insert("iri", ni.0.0.as_ref());
                    this_kind = Kind::NamedIndividual;
                    context.insert("kind", "named-individual")
                }
                Component::AnnotationAssertion(aa) => match aa.ann.ap.0.as_ref() {
                    "http://www.w3.org/2000/01/rdf-schema#label" => context.insert(
                        "label",
                        &unpack_annotation_value(&aa.ann.av).unwrap_or(iri.to_string()),
                    ),
                    "http://www.w3.org/2004/02/skos/core#definition" => context.insert(
                        "definition",
                        &unpack_annotation_value(&aa.ann.av).unwrap_or(iri.to_string()),
                    ),
                    "http://www.w3.org/2004/02/skos/core#example" => context.insert(
                        "example",
                        &unpack_annotation_value(&aa.ann.av).unwrap_or(iri.to_string()),
                    ),
                    _ => match unpack_annotation_value(&aa.ann.av) {
                        Some(vv) => {
                            let label = match self.prefix_mapping.shrink_iri(aa.ann.ap.0.as_ref()) {
                                Ok(s) => s.into(),
                                Err(_) => aa.ann.ap.0.to_string(),
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
                        let child_display =
                            self.build_entity_display(subc.0.clone(), &self.settings.ontology.iri);
                        sub_entities.push(DisplayComp::Simple(child_display))
                    } else if &subc.0 == iri {
                        let parent_display = self.build_entity_display(spc.0.clone(), b);
                        super_entities.push(DisplayComp::Simple(parent_display));
                    }
                }
                Component::SubClassOf(SubClassOf {
                    sup,
                    sub: ClassExpression::Class(subc),
                }) => {
                    if &subc.0 == iri {
                        let class_display = self.unpack_class_expression(sup.clone(), Some(b));
                        super_entities.push(class_display);
                    }
                }
                Component::SubClassOf(SubClassOf {
                    sup: ClassExpression::Class(supc),
                    sub,
                }) => {
                    if &supc.0 == iri {
                        let class_display = self.unpack_class_expression(sub.clone(), Some(b));
                        sub_entities.push(class_display);
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
                        let child_display = self.build_entity_display(sub.0.clone(), b);
                        sub_entities.push(DisplayComp::Simple(child_display))
                    } else if &sup.0 == iri {
                        let parent_display = self.build_entity_display(sub.0.clone(), b);
                        super_entities.push(DisplayComp::Simple(parent_display));
                    }
                }
                Component::SubDataPropertyOf(_) => (),
                Component::EquivalentClasses(EquivalentClasses(ecs)) => {
                    let ecx: Vec<DisplayComp> = ecs
                        .iter()
                        .map(|e| self.unpack_class_expression(e.clone(), Some(b)))
                        .filter(|ex| {
                            if let DisplayComp::Simple(e) = ex {
                                !(&e.iri == iri.as_ref())
                            } else {
                                true
                            }
                        })
                        .collect();
                    equivalent_classes.extend(ecx)
                }
                Component::EquivalentObjectProperties(_) => (),
                Component::EquivalentDataProperties(_) => (),
                Component::InverseObjectProperties(InverseObjectProperties(iop, iiop)) => {
                    if &iop.0 == iri {
                        let op_display = self.build_entity_display(iiop.0.clone(), b);
                        inverse_ops.push(DisplayComp::Simple(op_display));
                    } else if &iiop.0 == iri {
                        let op_display = self.build_entity_display(iop.0.clone(), b);
                        inverse_ops.push(DisplayComp::Simple(op_display));
                    }
                }
                Component::ObjectPropertyRange(ObjectPropertyRange {
                    ope: ObjectPropertyExpression::ObjectProperty(ObjectProperty(ii)),
                    ce,
                }) => {
                    if ii == iri {
                        let ce_display = self.unpack_class_expression(ce.clone(), Some(b));
                        context.insert("op_range", &ce_display);
                    }
                }
                Component::ObjectPropertyDomain(ObjectPropertyDomain {
                    ope: ObjectPropertyExpression::ObjectProperty(ObjectProperty(ii)),
                    ce,
                }) => {
                    if ii == iri {
                        let ce_display = self.unpack_class_expression(ce.clone(), Some(b));
                        context.insert("op_domain", &ce_display);
                    }
                }
                Component::DisjointClasses(_) => (),
                Component::DisjointObjectProperties(_) => (),
                Component::DisjointDataProperties(_) => (),
                Component::AnnotationPropertyRange(_) => (),
                Component::AnnotationPropertyDomain(_) => (),
                Component::ClassAssertion(ClassAssertion {
                    ce,
                    i: Individual::Named(ind),
                }) => {
                    if &ind.0 == iri {
                        let cexp = self.unpack_class_expression(ce.clone(), Some(b));
                        class_assertions.push(cexp);
                    }
                }
                _ => (),
            }
        }
        if super_entities.len() > 0 {
            context.insert("super_classes", &super_entities);
        }
        if sub_entities.len() > 0 {
            context.insert("sub_classes", &sub_entities);
        }
        if inverse_ops.len() > 0 {
            context.insert("inverse_ops", &inverse_ops);
        }
        if equivalent_classes.len() > 0 {
            context.insert("equivalent_classes", &equivalent_classes);
        }
        if class_assertions.len() > 0 {
            context.insert("class_assertions", &class_assertions);
        }
        context.insert("annotations", &annotations);
        match this_kind {
            Kind::Class => self
                .templates
                .render("entity.html", &context)
                .wrap_err("Could not render class page"),
            Kind::ObjectProperty => self
                .templates
                .render("entity.html", &context)
                .wrap_err("Could not render object property page"),
            Kind::AnnotationProperty => self
                .templates
                .render("entity.html", &context)
                .wrap_err("Could not render ann prop page"),
            Kind::NamedIndividual => self
                .templates
                .render("entity.html", &context)
                .wrap_err("Could not render ann prop page"),
            Kind::Undefined => {
                Err(tera::Error::msg("Not implemented")).wrap_err("Unkown entity kind")
            }
        }
    }

    fn render_all_declarations_html(
        &mut self,
        base: Option<&str>,
    ) -> Result<HashMap<IRI<A>, String>> {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri.clone(),
        };
        let mut declaration_hm: HashMap<IRI<A>, String> = HashMap::new();
        for cl in self.get_iris_for_declaration(ComponentKind::DeclareClass) {
            let rendered_page = self.render_declaration_iri_html(&cl, Some(b))?;
            match declaration_hm.entry(cl) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for ni in self.get_iris_for_declaration(ComponentKind::DeclareNamedIndividual) {
            let rendered_page = self.render_declaration_iri_html(&ni, Some(b))?;
            match declaration_hm.entry(ni) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for dp in self.get_iris_for_declaration(ComponentKind::DeclareDataProperty) {
            let rendered_page = self.render_declaration_iri_html(&dp, Some(b))?;
            match declaration_hm.entry(dp) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for op in self.get_iris_for_declaration(ComponentKind::DeclareObjectProperty) {
            let rendered_page = self.render_declaration_iri_html(&op, Some(b))?;
            match declaration_hm.entry(op) {
                Entry::Occupied(o) => println!("{:?}", o),
                Entry::Vacant(v) => {
                    v.insert(rendered_page);
                }
            }
        }
        for ap in self.get_iris_for_declaration(ComponentKind::DeclareAnnotationProperty) {
            let rendered_page = self.render_declaration_iri_html(&ap, Some(b))?;
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
        self.ontology
            .component_for_kind(component_kind)
            .map(|dc| match &dc.component {
                Component::DeclareClass(dc) => Some(dc.0.0.clone()),
                Component::DeclareNamedIndividual(ni) => Some(ni.0.0.clone()),
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

    fn render_metadata_html(&mut self, base: Option<&str>) -> Result<String> {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri.clone(),
        };
        let mut context = TeraContext::default();
        let mut contributors: Vec<OntologyAnnotation> = vec![];
        let mut annotations: Vec<OntologyAnnotation> = vec![];
        for oid in self.ontology.component_for_kind(ComponentKind::OntologyID) {
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
        for oann in self
            .ontology
            .component_for_kind(ComponentKind::OntologyAnnotation)
        {
            if let Component::OntologyAnnotation(oa) = &oann.component {
                let ann = match unpack_annotation_value(&oa.0.av) {
                    Some(vv) => {
                        let label = match self.prefix_mapping.shrink_iri(oa.0.ap.0.as_ref()) {
                            Ok(s) => s.into(),
                            Err(_) => oa.0.ap.0.to_string(),
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
        let entity_tree = match self.collect_entity_tree(Some(b)) {
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
        Ok(self.templates.render("ontology.html", &context)?)
    }

    fn collect_entity_tree(&mut self, base: Option<&str>) -> Result<SideBar> {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri,
        };
        let mut side_bar = SideBar::default();
        let scos: Vec<AnnotatedComponent<A>> = self
            .ontology
            .component_for_kind(ComponentKind::DeclareClass)
            .map(|x| x.clone())
            .collect();
        for sco in scos {
            match &sco.component {
                Component::DeclareClass(DeclareClass(Class(ii))) => {
                    if ii.contains(&self.settings.ontology.iri) {
                        let class_display = self.build_entity_display(ii.clone(), b);
                        side_bar.classes.push(class_display)
                    }
                }
                _ => (),
            }
        }
        let niss: Vec<AnnotatedComponent<A>> = self
            .ontology
            .component_for_kind(ComponentKind::DeclareNamedIndividual)
            .map(|x| x.clone())
            .collect();
        for nis in niss {
            match &nis.component {
                Component::DeclareNamedIndividual(DeclareNamedIndividual(NamedIndividual(ii))) => {
                    if ii.contains(&self.settings.ontology.iri) {
                        let i_display = self.build_entity_display(ii.clone(), b);
                        side_bar.named_individuals.push(i_display)
                    }
                }
                _ => (),
            }
        }
        let dops: Vec<AnnotatedComponent<A>> = self
            .ontology
            .component_for_kind(ComponentKind::DeclareObjectProperty)
            .map(|x| x.clone())
            .collect();
        for dop in dops {
            match &dop.component {
                Component::DeclareObjectProperty(DeclareObjectProperty(ObjectProperty(ii))) => {
                    if ii.contains(&self.settings.ontology.iri) {
                        let op_display = self.build_entity_display(ii.clone(), b);
                        side_bar.object_props.push(op_display)
                    }
                }
                _ => (),
            }
        }
        let daps: Vec<AnnotatedComponent<A>> = self
            .ontology
            .component_for_kind(ComponentKind::DeclareAnnotationProperty)
            .map(|x| x.clone())
            .collect();
        for dap in daps {
            match &dap.component {
                Component::DeclareAnnotationProperty(DeclareAnnotationProperty(
                    AnnotationProperty(ii),
                )) => {
                    if ii.contains(&self.settings.ontology.iri) {
                        let ap_display = self.build_entity_display(ii.clone(), b);
                        side_bar.annotation_props.push(ap_display)
                    }
                }
                _ => (),
            }
        }
        let ddps: Vec<AnnotatedComponent<A>> = self
            .ontology
            .component_for_kind(ComponentKind::DeclareDataProperty)
            .map(|x| x.clone())
            .collect();
        for ddp in ddps {
            match &ddp.component {
                Component::DeclareDataProperty(dp) => {
                    let class_iri = &dp.0.0;
                    if class_iri.contains(&self.settings.ontology.iri) {
                        let iri_string = class_iri.to_string();
                        let class_label =
                            self.label_map.get(class_iri).unwrap_or(&iri_string).clone();
                        let class_identifier = match self.prefix_mapping.shrink_iri(class_iri) {
                            Ok(r) => r.to_string(),
                            Err(_) => class_iri.to_string(),
                        };
                        side_bar.data_props.push(EntityDisplay::new(
                            iri_string,
                            class_identifier,
                            class_label,
                        ))
                    }
                }
                _ => (),
            }
        }
        Ok(side_bar)
    }

    fn unpack_class_expression(&self, ce: ClassExpression<A>, base: Option<&str>) -> DisplayComp {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri,
        };
        match ce {
            ClassExpression::Class(class) => {
                let disp = self.build_entity_display(class.0.clone(), b);
                DisplayComp::Simple(disp)
            }
            ClassExpression::ObjectIntersectionOf(class_expressions) => {
                let v: Vec<DisplayComp> = class_expressions
                    .iter()
                    .map(|ce| self.unpack_class_expression(ce.clone(), Some(b)))
                    .collect();
                DisplayComp::And(GroupDisplay(v))
            }
            ClassExpression::ObjectUnionOf(class_expressions) => {
                let v: Vec<DisplayComp> = class_expressions
                    .iter()
                    .map(|ce| self.unpack_class_expression(ce.clone(), Some(b)))
                    .collect();
                DisplayComp::Or(GroupDisplay(v))
            }
            ClassExpression::ObjectComplementOf(class_expression) => {
                let ce = self.unpack_class_expression(*class_expression, Some(b));
                DisplayComp::Not(Box::new(ce))
            }
            ClassExpression::ObjectOneOf(_) => todo!("Not implemented: ObjectOneOf"),
            ClassExpression::ObjectSomeValuesFrom { ope, bce } => {
                let ope = Box::new(self.unpack_object_property_expression(ope, Some(b)));
                let ce = Box::new(self.unpack_class_expression(*bce, Some(b)));
                DisplayComp::Some(RelDisplay { rel: ope, ce })
            }
            ClassExpression::ObjectAllValuesFrom { ope, bce } => {
                let ope = Box::new(self.unpack_object_property_expression(ope, Some(b)));
                let ce = Box::new(self.unpack_class_expression(*bce, Some(b)));
                DisplayComp::All(RelDisplay { rel: ope, ce })
            }
            ClassExpression::ObjectHasValue {
                ope,
                i: Individual::Named(ind),
            } => {
                let op = self.unpack_object_property_expression(ope, Some(b));
                let ce = self.build_entity_display(ind.0, b);
                DisplayComp::Value(RelDisplay {
                    rel: Box::new(op),
                    ce: Box::new(DisplayComp::Simple(ce)),
                })
            }
            ClassExpression::ObjectHasValue {
                i: horned_owl::model::Individual::Anonymous(_),
                ..
            } => todo!(),
            ClassExpression::ObjectHasSelf(_) => {
                todo!("Not implemented: ObjectHasSelf")
            }
            ClassExpression::ObjectMinCardinality {
                n: _,
                ope: _,
                bce: _,
            } => {
                todo!("Not implemented: ObjectMinCardinality")
            }
            ClassExpression::ObjectMaxCardinality {
                n: _,
                ope: _,
                bce: _,
            } => {
                todo!("Not implemented: ObjectMaxCardinality")
            }
            ClassExpression::ObjectExactCardinality {
                n: _,
                ope: _,
                bce: _,
            } => {
                todo!("Not implemented: ObjectExactCardinality")
            }
            ClassExpression::DataSomeValuesFrom { dp: _, dr: _ } => {
                todo!("Not implemented: DataSomeValuesFrom")
            }
            ClassExpression::DataAllValuesFrom { dp: _, dr: _ } => {
                todo!("Not implemented: DataAllValuesFrom")
            }
            ClassExpression::DataHasValue { dp, l } => {
                let dpd = self.build_entity_display(dp.0, b);
                let value = unpack_literal(l);
                DisplayComp::Data(DPDisplay {
                    dp: Box::new(DisplayComp::Simple(dpd)),
                    value,
                })
            }
            ClassExpression::DataMinCardinality { n: _, dp: _, dr: _ } => {
                todo!("Not implemented: DataMinCardinality")
            }
            ClassExpression::DataMaxCardinality { n: _, dp: _, dr: _ } => {
                todo!("Not implemented: DataMaxCardinality")
            }
            ClassExpression::DataExactCardinality { n: _, dp: _, dr: _ } => {
                todo!("Not implemented: DataExactCardinality")
            }
        }
    }

    fn build_entity_display(&self, iri: IRI<A>, base: &str) -> EntityDisplay {
        let entity_id = if iri.contains(base) {
            iri.replace(base, "")
        } else {
            iri.to_string()
        };
        let entity_label = match self.label_map.get(&iri) {
            Some(l) => l.clone(),
            None => match self.prefix_mapping.shrink_iri(iri.as_ref()) {
                Ok(r) => r.to_string(),
                Err(_) => iri.to_string(),
            },
        };
        EntityDisplay::new(iri.to_string(), entity_id, entity_label)
    }

    fn unpack_object_property_expression(
        &self,
        ope: ObjectPropertyExpression<A>,
        base: Option<&str>,
    ) -> DisplayComp {
        let b = match base {
            Some(s) => s,
            None => &self.settings.ontology.iri,
        };
        match ope {
            ObjectPropertyExpression::ObjectProperty(object_property) => {
                let op_display = self.build_entity_display(object_property.0.clone(), b);
                DisplayComp::Simple(op_display)
            }
            ObjectPropertyExpression::InverseObjectProperty(object_property) => {
                let op_display = self.build_entity_display(object_property.0.clone(), b);
                DisplayComp::Simple(op_display)
            }
        }
    }
}

impl<A: ForIRI, AA: ForIndex<A>> OntologyRender<A, AA> {
    pub fn new_with_settings(settings: Settings) -> Result<Self> {
        let dir = if let Some(d) = &settings.ontology.source {
            d
        } else {
            return Err(eyre::eyre!("Expected source file"));
        };
        let build: Build<A> = Build::new();
        let f = File::open(dir)?;
        let reader = BufReader::new(f);
        let r = read_with_build(reader, &build);
        assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
        let (o, mut prefix_mapping) = r.ok().unwrap();
        let mut ontology: IRIMappedOntology<A, AA> = IRIMappedOntology::from(o);
        let label_map = get_label_hashmap(&mut ontology);
        prefix_mapping.set_default(&settings.ontology.iri);
        if let Some(imports) = &settings.import {
            for imp in imports.iter() {
                if let Some(p) = &imp.suffix {
                    match prefix_mapping.add_prefix(&p, &imp.iri) {
                        Ok(_) => (),
                        Err(_) => return Err(eyre::eyre!("Error when adding prefixes")),
                    };
                }
            }
        }
        let templates: Tera = {
            let templates_dir = match &settings.templates {
                Some(dir) => {
                    let mut out = String::from(dir.trim_end_matches("/"));
                    out.push_str("/**/*.html");
                    out
                }
                None => {
                    return Err(eyre::eyre!(
                        "Templates not defined, set templates directory"
                    ));
                }
            };
            let mut tera = match Tera::new(&templates_dir) {
                Ok(t) => t,
                Err(e) => return Err(eyre::eyre!("Parsing error(s): {}", e)),
            };
            tera.autoescape_on(vec![".html", ".sql"]);
            tera
        };
        Ok(OntologyRender {
            ontology,
            prefix_mapping,
            label_map,
            settings,
            templates,
        })
    }
}

fn unpack_literal<A: ForIRI>(l: Literal<A>) -> String {
    match l {
        Literal::Simple { literal } => literal,
        Literal::Language { literal, lang: _ } => literal,
        Literal::Datatype {
            literal,
            datatype_iri: _,
        } => literal,
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

fn get_label_hashmap<A, AA>(ontology: &mut IRIMappedOntology<A, AA>) -> HashMap<IRI<A>, String>
where
    A: ForIRI,
    AA: ForIndex<A>,
{
    let mut label_map: HashMap<IRI<A>, String> = HashMap::new();

    for aa in ontology.component_for_kind(ComponentKind::AnnotationAssertion) {
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
