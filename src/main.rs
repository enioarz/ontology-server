use dotenvy::dotenv;
use eyre::Result;
use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::ontology::iri_mapped::ArcIRIMappedOntology;
use hyper_ontology::render_html::{IRIMappedRenderHTML, ONTOLOGY_IRI};
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    dotenv()?;
    let _suff = env::var("ONTOLOGY_SUFFIX")?;
    let dir = env::var("ONTOLOGY_DIR")?;
    let build = Build::new_arc();
    let f = File::open(dir)?;
    let reader = BufReader::new(f);
    let r = read_with_build(reader, &build);
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, mut pm) = r.ok().unwrap();
    let mut oc: ArcIRIMappedOntology = ArcIRIMappedOntology::from(o);
    pm.set_default(ONTOLOGY_IRI.as_str());
    pm.add_prefix("skos", "http://www.w3.org/2004/02/skos/core#")
        .unwrap();
    pm.add_prefix("dct", "http://purl.org/dc/terms/").unwrap();
    pm.add_prefix("dce", "http://purl.org/dc/elements/1.1/")
        .unwrap();
    // pm.add_prefix("live", &base_url).unwrap();
    let hm = oc.render_all_declarations_html(Some(&pm))?;
    fs::create_dir_all("public").unwrap_or(println!("Folder already exist"));
    for (k, v) in hm.iter() {
        match pm.shrink_iri(k) {
            Ok(i) => {
                let iri_parts: Vec<String> = i
                    .to_string()
                    .split(":")
                    .map(|s: &str| s.to_string())
                    .collect();
                let prefix_len = iri_parts.len();
                if prefix_len == 1 {
                    fs::write(format!("public/{}.html", &iri_parts[0]), v).unwrap();
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
    copy_dir_all("static", "public/static")?;
    Ok(())
}
