/** Client-side medical PDF: no patient data sent to another PDF service. */
import { PDFDocument, StandardFonts, rgb, type PDFFont, type PDFPage } from 'pdf-lib';
export interface PatientMetadata {surname:string;forename:string;dateOfBirth:string;practitioner?:string}
export function medicalFilename(patient:PatientMetadata,date=new Date()):string{
  const letters=(v:string)=>v.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toUpperCase().replace(/[^A-Z]/g,'').slice(0,3);
  const a=letters(patient.surname),b=letters(patient.forename);
  const day=[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-');
  return (a&&b?a+b+'_':'CONSULTATION_')+day+'.pdf';
}
export function structuredMedicalBody(text:string):string{
  const matched=text.match(/^## Structured notes\s*([\s\S]*?)(?=^## Transcript\s*$|(?![\s\S]))/m);
  if(matched?.[1]?.trim())return matched[1].trim();
  return '## Transcription non structurée - à vérifier\n\n'+text.replace(/^---\s*\n[\s\S]*?\n---\s*\n/,'').trim();
}
function safe(text:string):string{
  return text.replace(/[*_]/g,'').replace(/[\u2018\u2019]/g,"'").replace(/[\u201c\u201d]/g,'"')
    .replace(/[\u2013\u2014]/g,'-').replace(/\u2022/g,'-').replace(/\u2026/g,'...')
    .replace(/\u00a0/g,' ').replace(/[\u2000-\u200f\uFE0F]/g,'')
    .replace(/[\uD800-\uDBFF][\uDC00-\uDFFF]/g,'')
    .replace(/[\u0100-\uFFFF]/g,'?');
}
function wrap(text:string,font:PDFFont,size:number,width:number):string[]{
  const lines:string[]=[];let line='';
  for(const word of safe(text).split(/\s+/).filter(Boolean)){
    let chunks=[word];
    if(font.widthOfTextAtSize(word,size)>width){
      chunks=[];let piece='';
      for(const char of word){if(piece&&font.widthOfTextAtSize(piece+char,size)>width){chunks.push(piece);piece='';}piece+=char;}
      if(piece)chunks.push(piece);
    }
    for(const chunk of chunks){const next=line?line+' '+chunk:chunk;if(line&&font.widthOfTextAtSize(next,size)>width){lines.push(line);line=chunk;}else line=next;}
  }
  if(line)lines.push(line);return lines;
}
export async function createMedicalPdf(markdown:string,patient:PatientMetadata,date=new Date()):Promise<Blob>{
  const pdf=await PDFDocument.create();
  pdf.setTitle('Compte rendu de consultation');pdf.setCreator('ScribeWatch');
  const regular=await pdf.embedFont(StandardFonts.Helvetica);
  const bold=await pdf.embedFont(StandardFonts.HelveticaBold);
  const W=595.28,H=841.89,M=49,contentWidth=W-2*M;
  const navy=rgb(.11,.19,.24),teal=rgb(.08,.4,.42),muted=rgb(.42,.46,.5);
  let page:PDFPage;let y=0;const pages:PDFPage[]=[];
  const addPage=()=>{
    page=pdf.addPage([W,H]);pages.push(page);y=H-53;
    page.drawText('SCRIBEWATCH / DOCUMENT CLINIQUE',{x:M,y,size:8,font:bold,color:teal});
    page.drawLine({start:{x:M,y:y-11},end:{x:W-M,y:y-11},thickness:.8,color:rgb(.82,.86,.87)});
    y-=34;
  };
  const ensure=(height:number)=>{if(y-height<67)addPage();};
  const draw=(text:string,style:'title'|'section'|'body'|'bullet'|'table'='body')=>{
    const size=style==='title'?17:style==='section'?12:10.3;
    const font=style==='body'||style==='bullet'?regular:bold;
    const indent=style==='bullet'?12:0,leading=size*1.43;
    const lines=wrap(text,font,size,contentWidth-indent);
    if(!lines.length)return;
    ensure(leading*2+8);
    if(style==='title'||style==='section'){y-=9;ensure(16);page.drawLine({start:{x:M,y:y-3},end:{x:M+18,y:y-3},thickness:2,color:teal});y-=14;}
    for(const line of lines){ensure(leading);page.drawText(line,{x:M+indent,y,size,font,color:style==='body'||style==='bullet'?navy:teal});y-=leading;}
    y-=style==='title'||style==='section'?9:4;
  };
  addPage();
  draw('Compte rendu de consultation','title');
  draw('Date de consultation : '+new Intl.DateTimeFormat('fr-FR',{dateStyle:'long'}).format(date));
  if(patient.surname.trim()||patient.forename.trim())draw('Patient : '+patient.forename.trim()+' '+patient.surname.trim().toUpperCase(),'section');
  if(patient.dateOfBirth)draw('Date de naissance : '+patient.dateOfBirth);
  if(patient.practitioner?.trim())draw('Praticien : '+patient.practitioner.trim());
  y-=14;
  const body=structuredMedicalBody(markdown);
  for(const raw of body.split('\n')){
    const line=raw.trim();if(!line){y-=3;continue;}
    if(/^#{1,3}\s/.test(line))draw(line.replace(/^#{1,3}\s+/,''),'section');
    else if(/^[-*]\s/.test(line)||/^\d+[.)]\s/.test(line))draw('- '+line.replace(/^(?:[-*]|\d+[.)])\s+/,''),'bullet');
    else if(line.startsWith('|')){if(/^\|[\s:|-]+\|?$/.test(line))continue;draw(line.split('|').map(x=>x.trim()).filter(Boolean).join(' | '),'table');}
    else draw(line);
  }
  for(let i=0;i<pages.length;i++){
    const p=pages[i];
    p.drawLine({start:{x:M,y:53},end:{x:W-M,y:53},thickness:.7,color:rgb(.78,.83,.84)});
    p.drawText('Brouillon - validation obligatoire par le praticien',{x:M,y:37,size:8,font:regular,color:muted});
    const n='Page '+(i+1)+' / '+pages.length;
    p.drawText(n,{x:W-M-bold.widthOfTextAtSize(n,8),y:37,size:8,font:bold,color:muted});
  }
  return new Blob([new Uint8Array(await pdf.save())],{type:'application/pdf'});
}
