import { planPageSelection } from './page-selection.js';
export function createConversionUi({ t, invoke, getDocument, preparePdf, showStatus, errorText }) {
 const ui = Object.fromEntries(['modal','backdrop','title','help','pages-row','pages','formats','dpi-row','dpi','quality-row','quality','summary','error','confirm','cancel','fields','result','result-path','reveal'].map(key=>[key,document.getElementById(`conversion-${key}`)]));
 let context = null;
 const format = () => ui.formats.querySelector('input:checked')?.value || 'png';
 const translateError = error => error?.key ? t(error.key,error.value) : errorText(error);
 const selection = () => context.kind==='image' ? [] : planPageSelection(ui.pages.value,context.pageCount,'selected').pages;
 function refresh() {
  if (!context || context.busy || context.result) return;
  ui['quality-row'].classList.toggle('hidden',context.kind==='text'||format()!=='jpg');
  try {
   const pages=selection();ui.confirm.disabled=false;ui.error.textContent='';
   ui.summary.textContent=context.kind==='text' ? t('conversionTextHint') : t('conversionFormatHint',format());
   if(context.kind==='pages')ui.summary.textContent=t('conversionPageCount',pages.length)+' '+ui.summary.textContent;
  }catch(error){ui.error.textContent=translateError(error);ui.confirm.disabled=true;ui.summary.textContent='';}
 }
 function close(){if(context?.busy)return;const focus=context?.focus;context=null;ui.modal.classList.add('hidden');ui.backdrop.classList.add('hidden');if(focus?.isConnected)focus.focus();}
 async function open(kind) {
  if(context)return;
  const focus=document.activeElement;
  let source=getDocument();
  if(kind==='image') {
   context={busy:true};
   try{source=await invoke('pick_conversion_image');}catch(error){showStatus(translateError(error),'error');context=null;return;}
   if(!source){context=null;return;}
  } else if(!source?.pageCount){showStatus(t('needPdfOpen'),'error');return;}
  context={...source,kind,focus,busy:false,result:null,requestId:Date.now().toString(36)+Math.random().toString(36).slice(2)};
  const titleKeys={pages:'conversionPdfImages',embedded:'conversionExtractImages',text:'conversionPdfText',image:'conversionImageFormat'};
  ui.title.textContent=t(titleKeys[kind]);ui.help.textContent=kind==='image'?t('conversionImageInfo',source.filename,source.width,source.height):t('conversionPdfInfo',source.filename,source.pageCount);
  ui['pages-row'].classList.toggle('hidden',kind==='image');ui.formats.classList.toggle('hidden',kind==='text');ui['dpi-row'].classList.toggle('hidden',kind!=='pages');
  ui.pages.value=source.pageCount>1?`1-${source.pageCount}`:'1';ui.pages.placeholder=t('splitPlaceholder');
  ui.dpi.value='150';ui.quality.value='90';ui.formats.querySelector('[value="png"]').checked=true;
  ui.fields.classList.remove('hidden');ui.result.classList.add('hidden');ui.confirm.classList.remove('hidden');ui.confirm.textContent=t('conversionSave');ui.cancel.textContent=t('cancel');ui.reveal.textContent=t('splitReveal');
  ui.error.textContent='';for(const input of ui.fields.querySelectorAll('input,select'))input.disabled=false;ui.cancel.disabled=false;
  ui.modal.classList.remove('hidden');ui.backdrop.classList.remove('hidden');refresh();
  if(kind==='image')ui.formats.querySelector('input:checked').focus();else{ui.pages.focus();ui.pages.select();}
 }
 function ensureDocument(source){const current=getDocument();if(current?.id!==source.id||current?.pageCount!==source.pageCount||current?.sourcePath!==source.sourcePath)throw new Error(t('conversionDocumentChanged'));}
 async function submit(){
  if(!context||context.busy||context.result)return;
  let pages;try{pages=selection();}catch(error){ui.error.textContent=translateError(error);return;}
  const current=context;current.busy=true;ui.error.textContent='';ui.summary.textContent=t('conversionPreparing');
  ui.confirm.disabled=ui.cancel.disabled=true;for(const input of ui.fields.querySelectorAll('input,select'))input.disabled=true;
  try{
   const options={format:format(),quality:Number(ui.quality.value)};
   let result;
   if(current.kind==='image'){result=await invoke('convert_image_dialog',{bytes:current.bytes,filename:current.filename,sourcePath:current.path,...options});}
   else{ensureDocument(current);const bytes=await preparePdf();ensureDocument(current);result=await invoke('export_pdf_conversion',{bytes,pages,kind:current.kind,filename:current.filename,sourcePath:current.sourcePath,...options,dpi:Number(ui.dpi.value),requestId:current.requestId});}
   if(result){current.result=result;ui.fields.classList.add('hidden');ui.result.classList.remove('hidden');ui.confirm.classList.add('hidden');ui['result-path'].textContent=result.directory||result.paths[0];ui.summary.textContent=t('conversionDone',result.paths.length);ui.cancel.textContent=t('done');}
  }catch(error){ui.error.textContent=translateError(error);}
  finally{current.busy=false;ui.cancel.disabled=false;for(const input of ui.fields.querySelectorAll('input,select'))input.disabled=false;
   if(!current.result){if(!ui.error.textContent)refresh();else{ui.summary.textContent='';ui.confirm.disabled=false;}}
  }
 }
 ui.fields.addEventListener('input',refresh);ui.fields.addEventListener('change',refresh);ui.confirm.addEventListener('click',()=>void submit());ui.cancel.addEventListener('click',close);ui.backdrop.addEventListener('click',close);
 ui.reveal.addEventListener('click',()=>{if(context?.result)void invoke('reveal_file_in_folder',{path:context.result.directory||context.result.paths[0]}).catch(error=>{ui.error.textContent=translateError(error);});});
 ui.modal.addEventListener('keydown',event=>{
  if(event.key==='Escape'){event.preventDefault();event.stopPropagation();close();}
  if(event.key==='Enter'&&event.target===ui.pages){event.preventDefault();void submit();}
  if(event.key==='Tab'){const controls=[...ui.modal.querySelectorAll('input,select,button')].filter(e=>!e.disabled&&e.getClientRects().length);if(event.shiftKey&&event.target===controls[0]){event.preventDefault();controls.at(-1)?.focus();}else if(!event.shiftKey&&event.target===controls.at(-1)){event.preventDefault();controls[0]?.focus();}}
 });
 return {open,progress:payload=>{if(context?.busy&&payload?.requestId===context.requestId)ui.summary.textContent=t('conversionProgress',payload.completed,payload.total);}};
}
