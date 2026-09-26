#[path = "../src/pdf_split.rs"]
#[allow(dead_code)]
mod pdf_split;
use lopdf::{dictionary, Document, Object, Stream};
use std::fs;
use std::path::PathBuf;
use sofdocs_desktop::pdf_engine;

fn fixture() -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica"});
    let mut kids = Vec::new(); let mut fields = Vec::new();
    for n in 1..=5 {
        let content = doc.add_object(Stream::new(dictionary!{}, format!("BT /F1 18 Tf 60 700 Td (PAGE {n}) Tj ET").into_bytes()));
        let page = doc.new_object_id();
        let field = doc.add_object(dictionary! {"Type"=>"Annot", "Subtype"=>"Widget", "FT"=>"Tx", "T"=>Object::string_literal(format!("field{n}")), "V"=>Object::string_literal(format!("value{n}")), "P"=>page, "Rect"=>vec![60.into(),600.into(),200.into(),630.into()]});
        fields.push(field.into());
        doc.objects.insert(page,dictionary! {"Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()], "Contents"=>content, "Annots"=>vec![field.into()], "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}}.into());
        kids.push(page.into());
    }
    doc.objects.insert(pages,dictionary!{"Type"=>"Pages","Count"=>5,"Kids"=>kids}.into());
    let acro = doc.add_object(dictionary!{"Fields"=>fields});
    let root = doc.add_object(dictionary!{"Type"=>"Catalog","Pages"=>pages,"AcroForm"=>acro});
    doc.trailer.set("Root",root);let mut out=Vec::new();doc.save_to(&mut out).unwrap();out
}
struct Temp(PathBuf);
impl Temp { fn new() -> Self { let p=std::env::temp_dir().join(format!("slate-split-test-{}-{}",std::process::id(),rand::random::<u64>()));fs::create_dir(&p).unwrap();Self(p) } }
impl Drop for Temp { fn drop(&mut self) { let _=fs::remove_dir_all(&self.0); } }
#[test]
fn selected_pages_keep_text_order_and_only_their_form_fields() {
    let result=pdf_split::extract_selected(&fixture(),&[5,2,2]).unwrap();
    let doc=Document::load_mem(&result).unwrap();assert_eq!(doc.get_pages().len(),2);
    assert!(pdf_engine::analyze_pdf_page(&result,1).unwrap().blocks.iter().any(|b| b.text.contains("PAGE 2")));
    assert!(pdf_engine::analyze_pdf_page(&result,2).unwrap().blocks.iter().any(|b| b.text.contains("PAGE 5")));
    let form=doc.catalog().unwrap().get(b"AcroForm").unwrap().as_reference().unwrap();
    let fields=doc.get_dictionary(form).unwrap().get(b"Fields").unwrap().as_array().unwrap();
    let names:Vec<_>=fields.iter().map(|v|doc.get_dictionary(v.as_reference().unwrap()).unwrap().get(b"T").unwrap().as_str().unwrap().to_vec()).collect();
    assert_eq!(names,vec![b"field2".to_vec(),b"field5".to_vec()]);
}
#[test]
fn bulk_split_never_overwrites_existing_files_or_source() {
    let dir=Temp::new();let bytes=fixture();let source=dir.0.join("Исходный.pdf");fs::write(&source,&bytes).unwrap();
    assert!(pdf_split::write_single(&bytes,&[1],&source,Some(&source)).is_err());
    let groups=vec![vec![1,2],vec![3],vec![4,5]];
    let a=pdf_split::write_multiple(&bytes,&groups,&dir.0,"Исходный.pdf").unwrap();
    let b=pdf_split::write_multiple(&bytes,&groups,&dir.0,"Исходный.pdf").unwrap();
    assert_ne!(a.directory,b.directory);assert_eq!(a.paths.len(),3);
    for (path,count) in a.paths.iter().zip([2,1,2]) { assert_eq!(Document::load(path).unwrap().get_pages().len(),count); }
    assert_eq!(fs::read(&source).unwrap(),bytes);
}
#[test]
fn invalid_selections_create_nothing() {
    let dir=Temp::new();for groups in [vec![],vec![vec![]],vec![vec![0]],vec![vec![6]]] {
        assert!(pdf_split::write_multiple(&fixture(),&groups,&dir.0,"x.pdf").is_err());
    }
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(),0);
}
#[test]
fn filenames_cannot_escape_destination() {
    let name=pdf_split::suggested_name("../../test.pdf",&[1,2,3]);
    assert_eq!(name,"test_стр_1-3.pdf");
    assert!(!pdf_split::suggested_name("a:b\\c.pdf",&[1]).contains(['/', '\\', ':']));
}
