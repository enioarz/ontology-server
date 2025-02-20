use dotenvy::dotenv;
use eyre::Result;
use horned_owl::io::owx::reader::read_with_build;
use horned_owl::model::Build;
use horned_owl::ontology::iri_mapped::ArcIRIMappedOntology;
use hyper_ontology::render_html::IRIMappedRenderHTML;
use std::fs;
use std::path::Path;

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    dotenv()?;
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
    let build = Build::new_arc();
    let ont_s = include_str!("../ontology/bfo.owx");
    let r = read_with_build(ont_s.as_bytes(), &build);
    assert!(r.is_ok(), "Expected ontology, got failure:{:?}", r.err());
    let (o, mut pm) = r.ok().unwrap();
    let mut oc: ArcIRIMappedOntology = ArcIRIMappedOntology::from(o);
    pm.add_prefix("bfo", "http://purl.obolibrary.org/obo/")
        .unwrap();
    let hm = oc.render_all_declarations_html(Some(&pm))?;
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
    copy_dir_all("static", "public/static")?;
    Ok(())
}
