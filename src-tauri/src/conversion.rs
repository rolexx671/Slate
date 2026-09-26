//! Local conversion with real encoders, bounded raster dimensions and safe output files.
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use pdfium_render::prelude::PdfRenderConfig;
use serde::Serialize;
use sofdocs_desktop::{pdf_engine, pdf_ops, pdf_tools};
use std::{fs::{self, OpenOptions}, io::{Cursor, Write}, path::Path};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

fn err(e: impl std::fmt::Display) -> String { e.to_string() }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format { Jpg, Png, Webp, Tiff }
impl Format {
 pub fn parse(s: &str) -> Result<Self, String> { match s { "jpg" | "jpeg" => Ok(Self::Jpg), "png" => Ok(Self::Png), "webp" => Ok(Self::Webp), "tiff" | "tif" => Ok(Self::Tiff), _ => Err("Выберите JPG, PNG, WebP или TIFF.".into()) } }
 pub fn ext(self) -> &'static str { match self {Self::Jpg=>"jpg",Self::Png=>"png",Self::Webp=>"webp",Self::Tiff=>"tiff"} }
}
fn check_size(w:u32,h:u32) -> Result<(),String> {
 if w==0 || h==0 || w>16000 || h>16000 || u64::from(w)*u64::from(h)>40_000_000 { Err("Изображение слишком большое. Уменьшите разрешение: максимум 40 мегапикселей и 16 000 пикселей по стороне.".into()) } else {Ok(())}
}
pub fn decode_picture(raw:&[u8]) -> Result<DynamicImage,String> {
 let format=image::guess_format(raw).map_err(err)?;
 if !matches!(format,ImageFormat::Png|ImageFormat::Jpeg|ImageFormat::WebP|ImageFormat::Tiff) {return Err("Поддерживаются JPG, PNG, WebP и TIFF.".into());}
 if format==ImageFormat::Tiff && tiff::decoder::Decoder::new(Cursor::new(raw)).map_err(err)?.more_images() {return Err("Выберите одностраничный TIFF. Многостраничные TIFF пока не поддерживаются.".into());}
 if format==ImageFormat::WebP && image::codecs::webp::WebPDecoder::new(Cursor::new(raw)).map_err(err)?.has_animation() {return Err("Выберите статичное изображение WebP: анимация не поддерживается.".into());}
 if format==ImageFormat::Png && image::codecs::png::PngDecoder::new(Cursor::new(raw)).map_err(err)?.is_apng().map_err(err)? {return Err("Выберите обычный PNG: анимация APNG не поддерживается.".into());}
 let mut reader=ImageReader::with_format(Cursor::new(raw),format);
 let mut limits=image::Limits::default();limits.max_image_width=Some(16000);limits.max_image_height=Some(16000);limits.max_alloc=Some(256*1024*1024);reader.limits(limits);
 let mut decoder=reader.into_decoder().map_err(err)?;let (w,h)=decoder.dimensions();check_size(w,h)?;
 let orientation=decoder.orientation().map_err(err)?;
 let mut image=DynamicImage::from_decoder(decoder).map_err(err)?;image.apply_orientation(orientation);Ok(image)
}
pub fn encode_picture(image:&DynamicImage, format:Format, quality:u8, dpi:Option<u16>) -> Result<Vec<u8>,String> {
 check_size(image.width(),image.height())?;
 if !(1..=100).contains(&quality) {return Err("Качество JPG должно быть от 1 до 100.".into());}
 let rgba=image.to_rgba8();let mut out=Vec::new();
 match format {
  Format::Jpg=>{
   let mut rgb=image::RgbImage::new(rgba.width(),rgba.height());
   for (dst,src) in rgb.pixels_mut().zip(rgba.pixels()) {let a=u32::from(src[3]);for c in 0..3 {dst[c]=((u32::from(src[c])*a+255*(255-a)+127)/255) as u8;}}
   let mut encoder=image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out,quality);
   if let Some(dpi)=dpi {encoder.set_pixel_density(image::codecs::jpeg::PixelDensity::dpi(dpi));}
   encoder.encode_image(&rgb).map_err(err)?;
  }
  Format::Png=>{
   let mut encoder=png::Encoder::new(&mut out,rgba.width(),rgba.height());encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
   if let Some(dpi)=dpi {let ppm=(f64::from(dpi)/0.0254).round() as u32;encoder.set_pixel_dims(Some(png::PixelDimensions{xppu:ppm,yppu:ppm,unit:png::Unit::Meter}));}
   encoder.write_header().map_err(err)?.write_image_data(rgba.as_raw()).map_err(err)?;
  }
  Format::Webp=>{image::codecs::webp::WebPEncoder::new_lossless(&mut out).encode(rgba.as_raw(),rgba.width(),rgba.height(),image::ExtendedColorType::Rgba8).map_err(err)?;}
  Format::Tiff=>{
   let mut encoder=tiff::encoder::TiffEncoder::new(Cursor::new(&mut out)).map_err(err)?.with_compression(tiff::encoder::Compression::Deflate(Default::default()));
   let mut picture=encoder.new_image::<tiff::encoder::colortype::RGBA8>(rgba.width(),rgba.height()).map_err(err)?;
   if let Some(dpi)=dpi {
    picture.encoder().write_tag(tiff::tags::Tag::ResolutionUnit,2u16).map_err(err)?;
    picture.encoder().write_tag(tiff::tags::Tag::XResolution,tiff::encoder::Rational{n:u32::from(dpi),d:1}).map_err(err)?;
    picture.encoder().write_tag(tiff::tags::Tag::YResolution,tiff::encoder::Rational{n:u32::from(dpi),d:1}).map_err(err)?;
   }
   picture.write_data(rgba.as_raw()).map_err(err)?;
  }
 }
 Ok(out)
}
pub fn checked_images_to_pdf(images:Vec<Vec<u8>>) -> Result<Vec<u8>,String> {
 let normalized=images.iter().map(|raw|encode_picture(&decode_picture(raw)?,Format::Png,90,None)).collect::<Result<Vec<_>,_>>()?;
 pdf_tools::images_to_pdf(normalized)
}
pub fn validate_pages(bytes:&[u8], pages:&[u32], dpi:u16) -> Result<Vec<u32>,String> {
 if ![72,150,300].contains(&dpi) {return Err("Выберите разрешение 72, 150 или 300 dpi.".into());}
 let count=pdf_ops::page_count(bytes.to_vec())?;
 if pages.is_empty() || pages.iter().any(|&p|p==0||p>count) {return Err(format!("Укажите страницы от 1 до {count}."));}
 let mut pages=pages.to_vec();pages.sort_unstable();pages.dedup();Ok(pages)
}
pub fn render_image(bytes:&[u8], page:u32, dpi:u16) -> Result<DynamicImage,String> {
 validate_pages(bytes,&[page],dpi)?;
 let guard=pdf_engine::pdfium_guard()?;let doc=guard.load_pdf_from_byte_slice(bytes,None).map_err(err)?;
 let page=doc.pages().get((page-1) as i32).map_err(err)?;
 let width=(page.width().value*f32::from(dpi)/72.).round().max(1.) as u32;
 let height=(page.height().value*f32::from(dpi)/72.).round().max(1.) as u32;check_size(width,height)?;
 let rendered=page.render_with_config(&PdfRenderConfig::new().set_target_width(width as i32).render_form_data(true)).map_err(err)?.as_image().map_err(err)?;
 Ok(rendered)
}
pub fn extract_text(bytes:&[u8], pages:&[u32]) -> Result<String,String> {
 let pages=validate_pages(bytes,pages,150)?;let guard=pdf_engine::pdfium_guard()?;let doc=guard.load_pdf_from_byte_slice(bytes,None).map_err(err)?;
 let mut sections=Vec::new();for page in pages {sections.push(doc.pages().get((page-1) as i32).map_err(err)?.text().map_err(err)?.all());}
 let text=sections.join("\n\n");if text.trim().is_empty(){return Err("В выбранных страницах нет текстового слоя. Для скана сначала выполните распознавание текста (OCR).".into());}Ok(text)
}
#[derive(Debug,Serialize)]
pub struct ResultFiles {pub paths:Vec<String>, pub directory:Option<String>}
fn safe_stem(filename:&str)->String {let raw=Path::new(filename).file_stem().and_then(|s|s.to_str()).unwrap_or("Документ");let s:String=raw.chars().filter(|c|!c.is_control()&&!"/\\:*?\"<>|".contains(*c)).take(64).collect();if s.trim().is_empty(){"Документ".into()}else{s}}
fn same_file(a:&Path,b:&Path)->bool {a==b || matches!((fs::canonicalize(a),fs::canonicalize(b)),(Ok(a),Ok(b)) if a==b)}
pub fn write_atomic(data:&[u8],destination:&Path,source:Option<&Path>)->Result<(),String>{
 if source.is_some_and(|s|same_file(s,destination)){return Err("Выберите другое имя: исходный файл не должен перезаписываться.".into());}
 let parent=destination.parent().ok_or("Не найдена папка сохранения.")?;
 let temp=parent.join(format!(".slate-convert-{}-{}.tmp",std::process::id(),rand::random::<u64>()));
 let mut f=OpenOptions::new().write(true).create_new(true).open(&temp).map_err(err)?;
 let result=f.write_all(data).and_then(|_|f.sync_all()).and_then(|_|fs::rename(&temp,destination));
 if let Err(e)=result{let _=fs::remove_file(temp);return Err(format!("Не удалось сохранить файл: {e}"));}Ok(())
}
fn choose_targets(app:&AppHandle,names:&[String],ext:&str,filename:&str)->Result<Option<ResultFiles>,String>{
 if names.is_empty(){return Err("Нет файлов для сохранения.".into());}
 if names.len()==1 {
  let selected=app.dialog().file().set_title("Сохранить результат преобразования").add_filter(ext.to_uppercase(),&[ext]).set_file_name(&names[0]).blocking_save_file();
  let Some(selected)=selected else{return Ok(None)};let mut path=selected.into_path().map_err(err)?;path.set_extension(ext);
  Ok(Some(ResultFiles{paths:vec![path.to_string_lossy().into_owned()],directory:None}))
 }else{
  let selected=app.dialog().file().set_title("Выберите папку для результатов").blocking_pick_folder();let Some(selected)=selected else{return Ok(None)};let parent=selected.into_path().map_err(err)?;
  let base=format!("{} — {}",safe_stem(filename),ext.to_uppercase());let mut folder=None;
  for i in 1..10000 {let p=parent.join(if i==1 {base.clone()}else{format!("{base} ({i})")});match fs::create_dir(&p){Ok(())=>{folder=Some(p);break},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,Err(e)=>return Err(err(e))}}
  let folder=folder.ok_or("Не удалось создать папку для результатов.")?;
  Ok(Some(ResultFiles{paths:names.iter().map(|n|folder.join(n).to_string_lossy().into_owned()).collect(),directory:Some(folder.to_string_lossy().into_owned())}))
 }
}
fn cleanup_partial(result:&ResultFiles,written:&[String]) {if let Some(dir)=&result.directory {for p in written {let _=fs::remove_file(p);}let _=fs::remove_dir(dir);}}
#[derive(Clone,Serialize)]
#[serde(rename_all="camelCase")]
struct Progress {request_id:String,completed:usize,total:usize}
#[derive(Serialize)]
pub struct PickedImage {bytes:Vec<u8>,path:String,filename:String,width:u32,height:u32}
#[tauri::command]
pub async fn pick_conversion_image(app:AppHandle)->Result<Option<PickedImage>,String>{
 let selected=app.dialog().file().add_filter("Изображения",&["jpg","jpeg","png","webp","tif","tiff"]).blocking_pick_file();let Some(selected)=selected else{return Ok(None)};let path=selected.into_path().map_err(err)?;
 tauri::async_runtime::spawn_blocking(move||{let bytes=fs::read(&path).map_err(err)?;let image=decode_picture(&bytes)?;Ok(Some(PickedImage{width:image.width(),height:image.height(),filename:path.file_name().unwrap_or_default().to_string_lossy().into_owned(),path:path.to_string_lossy().into_owned(),bytes}))}).await.map_err(err)?
}
#[tauri::command]
pub async fn convert_image_dialog(app:AppHandle,bytes:Vec<u8>,filename:String,source_path:String,format:String,quality:u8)->Result<Option<ResultFiles>,String>{
 let format=Format::parse(&format)?;let name=format!("{}_преобразовано.{}",safe_stem(&filename),format.ext());
 let Some(target)=choose_targets(&app,&[name],format.ext(),&filename)? else{return Ok(None)};
 tauri::async_runtime::spawn_blocking(move||{let output=encode_picture(&decode_picture(&bytes)?,format,quality,None)?;write_atomic(&output,Path::new(&target.paths[0]),Some(Path::new(&source_path)))?;Ok(Some(target))}).await.map_err(err)?
}
#[tauri::command]
pub async fn export_pdf_conversion(app:AppHandle,bytes:Vec<u8>,pages:Vec<u32>,kind:String,filename:String,source_path:Option<String>,format:String,quality:u8,dpi:u16,request_id:String)->Result<Option<ResultFiles>,String>{
 let format=Format::parse(&format)?;if !["pages","embedded","text"].contains(&kind.as_str()){return Err("Неизвестный способ преобразования.".into());}
 let (bytes,pages,text,images)=tauri::async_runtime::spawn_blocking({let kind=kind.clone();move||{let pages=validate_pages(&bytes,&pages,dpi)?;
  let text=if kind=="text"{Some(extract_text(&bytes,&pages)?)}else{None};
  let images=if kind=="embedded"{pdf_tools::extract_images(&bytes)?.into_iter().filter(|image|pages.contains(&image.page)).collect::<Vec<_>>()}else{Vec::new()};
  if kind=="embedded"&&images.is_empty(){return Err("На выбранных страницах нет встроенных изображений.".into());}
  Ok::<_,String>((bytes,pages,text,images))}}).await.map_err(err)??;
 let ext=if kind=="text"{"txt"}else{format.ext()};let stem=safe_stem(&filename);
 let names=if kind=="text"{vec![format!("{stem}.txt")]}else if kind=="embedded"{images.iter().enumerate().map(|(i,img)|format!("{stem}_стр_{}_изображение_{:03}.{ext}",img.page,i+1)).collect()}else{pages.iter().map(|page|format!("{stem}_стр_{page:03}.{ext}")).collect()};
 let Some(target)=choose_targets(&app,&names,ext,&filename)? else{return Ok(None)};
 tauri::async_runtime::spawn_blocking(move||{
  let mut written=Vec::new();
  for (i,path) in target.paths.iter().enumerate(){
   let output=(||{if let Some(text)=&text{return Ok(text.as_bytes().to_vec())}let image=if kind=="embedded"{decode_picture(&images[i].png)?}else{render_image(&bytes,pages[i],dpi)?};encode_picture(&image,format,quality,if kind=="pages"{Some(dpi)}else{None})})();
   let saved=output.and_then(|data|write_atomic(&data,Path::new(path),source_path.as_deref().map(Path::new)));
   if let Err(error)=saved {cleanup_partial(&target,&written);return Err(error);}
   written.push(path.clone());let _=app.emit("slate-conversion-progress",Progress{request_id:request_id.clone(),completed:i+1,total:target.paths.len()});
  }
  Ok(Some(target))
 }).await.map_err(err)?
}
