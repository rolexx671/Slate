#[path = "../src/conversion.rs"]
#[allow(dead_code)]
mod conversion;
use conversion::{Format,encode_picture,decode_picture};
use image::{DynamicImage,RgbaImage,Rgba,ImageFormat};
use lopdf::{Document,Stream,dictionary};
use std::{fs,io::Cursor};

fn pdf() -> Vec<u8> {
 let mut doc=Document::with_version("1.5");let pages=doc.new_object_id();let font=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});let mut kids=Vec::new();
 for label in ["FIRST PAGE","SECOND PAGE"] {let stream=doc.add_object(Stream::new(dictionary!{},format!("BT /F1 12 Tf 10 28 Td ({label}) Tj ET").into_bytes()));let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),144.into(),72.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Contents"=>stream});kids.push(page.into());}
 doc.objects.insert(pages,dictionary!{"Type"=>"Pages","Count"=>2,"Kids"=>kids}.into());let root=doc.add_object(dictionary!{"Type"=>"Catalog","Pages"=>pages});doc.trailer.set("Root",root);let mut out=Vec::new();doc.save_to(&mut out).unwrap();out
}
#[test]
fn encoders_produce_real_formats_and_preserve_lossless_pixels() {
 let image=DynamicImage::ImageRgba8(RgbaImage::from_fn(8,5,|x,y|Rgba([(x*23)as u8,(y*47)as u8,170,if x%2==0{255}else{128}])));
 for (format,detected) in [(Format::Png,ImageFormat::Png),(Format::Webp,ImageFormat::WebP),(Format::Tiff,ImageFormat::Tiff),(Format::Jpg,ImageFormat::Jpeg)] {
  let bytes=encode_picture(&image,format,90,Some(150)).unwrap();assert_eq!(image::guess_format(&bytes).unwrap(),detected);
  let decoded=decode_picture(&bytes).unwrap();assert_eq!((decoded.width(),decoded.height()),(8,5));
  if format!=Format::Jpg {assert_eq!(decoded.to_rgba8(),image.to_rgba8());}
 }
}
#[test]
fn jpeg_uses_white_background_and_png_has_resolution_metadata() {
 let image=DynamicImage::ImageRgba8(RgbaImage::from_pixel(8,8,Rgba([0,0,0,0])));
 let jpeg=encode_picture(&image,Format::Jpg,90,Some(300)).unwrap();assert!(decode_picture(&jpeg).unwrap().to_rgb8().as_raw().iter().all(|v|*v>=250));
 let png=encode_picture(&image,Format::Png,90,Some(150)).unwrap();let reader=png::Decoder::new(Cursor::new(png)).read_info().unwrap();let dims=reader.info().pixel_dims.unwrap();assert_eq!(dims.xppu,5906);assert_eq!(dims.unit,png::Unit::Meter);
}
#[test]
fn pdf_resolution_selection_and_text_extraction() {
 let bytes=pdf();for (dpi,w,h) in [(72,144,72),(150,300,150),(300,600,300)] {let image=conversion::render_image(&bytes,1,dpi).unwrap();assert_eq!((image.width(),image.height()),(w,h));}
 let text=conversion::extract_text(&bytes,&[2]).unwrap();assert!(text.contains("SECOND PAGE"), "{text:?}");assert!(!text.contains("FIRST PAGE"));
 assert_eq!(conversion::validate_pages(&bytes,&[2,1,2],150).unwrap(),vec![1,2]);
 for pages in [vec![],vec![0],vec![3]]{assert!(conversion::validate_pages(&bytes,&pages,150).is_err());}
 assert!(conversion::render_image(&bytes,1,600).is_err());
}
#[test]
fn all_four_image_types_import_into_pdf_and_image_only_pdf_has_no_text() {
 let picture=DynamicImage::ImageRgba8(RgbaImage::from_pixel(12,8,Rgba([200,80,40,255])));
 let images=[Format::Jpg,Format::Png,Format::Webp,Format::Tiff].iter().map(|&f|encode_picture(&picture,f,90,None).unwrap()).collect();
 let result=conversion::checked_images_to_pdf(images).unwrap();assert_eq!(Document::load_mem(&result).unwrap().get_pages().len(),4);
 assert!(conversion::extract_text(&result,&[1]).unwrap_err().contains("OCR"));
}
#[test]
fn multipage_tiff_and_invalid_images_are_rejected_instead_of_losing_pages() {
 let mut bytes=Cursor::new(Vec::new());{let mut encoder=tiff::encoder::TiffEncoder::new(&mut bytes).unwrap();for _ in 0..2 {encoder.write_image::<tiff::encoder::colortype::RGB8>(1,1,&[1,2,3]).unwrap();}}
 assert!(decode_picture(&bytes.into_inner()).unwrap_err().contains("Многостраничные"));
 assert!(decode_picture(b"broken").is_err());assert!(Format::parse("gif").is_err());
 let image=DynamicImage::new_rgb8(1,1);assert!(encode_picture(&image,Format::Jpg,0,None).is_err());
}
#[test]
fn saving_cannot_overwrite_the_source() {
 let dir=std::env::temp_dir().join(format!("slate-conversion-test-{}",rand::random::<u64>()));fs::create_dir(&dir).unwrap();let source=dir.join("original.png");fs::write(&source,b"original").unwrap();
 assert!(conversion::write_atomic(b"changed",&source,Some(&source)).is_err());assert_eq!(fs::read(&source).unwrap(),b"original");
 let output=dir.join("copy.png");conversion::write_atomic(b"converted",&output,Some(&source)).unwrap();assert_eq!(fs::read(output).unwrap(),b"converted");fs::remove_dir_all(dir).unwrap();
}
