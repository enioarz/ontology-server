use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::ontology::iri_mapped::ArcIRIMappedOntology;
use hyper_ontology::render_html::IRIMappedRenderHTML;
use std::fs;

fn main() {
    let build = Build::new_arc();
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &build);
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, mut pm) = r.ok().unwrap();
    let mut oc: ArcIRIMappedOntology = ArcIRIMappedOntology::from(o);
    pm.add_prefix("bfo", "http://purl.obolibrary.org/obo/")
        .unwrap();
    oc.render_sidebar_html();
    let hm = oc.render_all_declarations_html(Some(&pm)).unwrap();
    fs::create_dir_all("public/BFO").unwrap_or(println!("Folder already exist"));
    for (k, v) in hm.iter() {
        match pm.shrink_iri(k) {
            Ok(i) => {
                let iri_parts: Vec<String> =
                    i.to_string().split(":").map(|s| s.to_string()).collect();
                let prefix = &iri_parts[0];
                match prefix.as_str() {
                    "bfo" => {
                        fs::write(format!("public/BFO/{}.html", &iri_parts[1]), v).unwrap();
                    }
                    _ => (),
                }
            }
            Err(_) => (),
        }
    }
    fs::write(
        "public/index.html",
        oc.render_metadata_html(Some(pm)).unwrap(),
    )
    .unwrap();
}
