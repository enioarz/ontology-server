use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::{Build, IRI};
use horned_owl::ontology::iri_mapped::ArcIRIMappedOntology;
use hyper_ontology::render_html::IRIMappedRenderHTML;

fn main() {
    let build = Build::new_arc();
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &build);
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, pm) = r.ok().unwrap();
    let mut oc: ArcIRIMappedOntology = ArcIRIMappedOntology::from(o);
    let classes: Vec<IRI<_>> = oc
        .component_for_kind(horned_owl::model::ComponentKind::DeclareClass)
        .map(|dc| {
            if let horned_owl::model::Component::DeclareClass(ddc) = &dc.component {
                Some(ddc.0.0.clone())
            } else {
                None
            }
        })
        .filter(|x| match x {
            Some(_) => true,
            None => false,
        })
        .map(|y| y.unwrap())
        .collect();
    for c in classes.into_iter() {
        let html = oc.render_declaration_iri_html(c, Some(&pm)).unwrap();
        println!("{}", html);
    }
    // let strng = build.iri("http://purl.obolibrary.org/obo/BFO_0000001");
    // let html = oc.render_iri_html(strng, Some(pm)).unwrap();
    // let html = oc.render_metadata_html(Some(pm)).unwrap();
    // println!("{}", html);
}
