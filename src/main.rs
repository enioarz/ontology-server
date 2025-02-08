use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::visitor::immutable::Walk;
use website::models::ClassCollection;

fn main() {
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &Build::new_string());
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, _) = r.ok().unwrap();

    let mut walk = Walk::new(ClassCollection::default());
    walk.set_ontology(&o);
    let mut hm: Vec<String> = walk
        .into_visit()
        .into_hashmap()
        .values()
        .map(|x| x.render().unwrap())
        .collect();
    println!("{}", hm[10]);
}
