use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::visitor::immutable::Walk;
use hyper_ontology::models::OntologyContent;

fn main() {
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &Build::new_string());
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, pm) = r.ok().unwrap();
    let oc = OntologyContent::new_with_prefix_mapping(pm);
    let mut walk = Walk::new(oc);
    walk.set_ontology(&o);
    let visit = walk.into_visit();
    let md = &visit.metadata.render_html().unwrap();
    let mut _hm: Vec<String> = visit
        .into_hashmap()
        .values()
        .map(|x| x.render_html().unwrap())
        .collect();

    println!("{}", md);
}
