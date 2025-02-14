use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::ontology::iri_mapped::RcIRIMappedOntology;
use hyper_ontology::IRIMappedRenderHTML;

fn main() {
    let build = Build::new_rc();
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &build);
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, pm) = r.ok().unwrap();
    let mut oc: RcIRIMappedOntology = RcIRIMappedOntology::from(o);

    // let strng = build.iri("http://purl.obolibrary.org/obo/BFO_0000001");
    // let html = oc.render_iri_html(strng, Some(pm)).unwrap();
    let html = oc.render_metadata_html(Some(pm)).unwrap();
    println!("{}", html);
}
