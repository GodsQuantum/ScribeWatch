/** Generic saved-report PDF: built into the browser, without Pandoc or WeasyPrint. */
import {PDFDocument,StandardFonts,rgb,type PDFFont,type PDFPage} from 'pdf-lib';
import {splitEditable} from './report-edit';
import type {SavedReport} from './types';

const W=595.28,H=841.89,M=49;

function printable(raw:string,font:PDFFont):string{
  const text=raw.replace(/!\[([^\]]*)\]\([^)]+\)/g,'$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g,'$1')
    .replace(/<[^>]*>/g,' ').replace(/\*\*|__/g,'').replace(/\x60/g,'');
  let result='';
  for(const character of text){
    try{font.encodeText(character);result+=character;}catch{result+='?';}
  }
  return result.replace(/\s+/g,' ').trim();
}

function wrap(raw:string,font:PDFFont,size:number,width:number):string[]{
  const result:string[]=[];let current='';
  const flush=()=>{if(current){result.push(current);current='';}};
  for(const word of raw.match(/\S+/g)||[]){
    const line=current?current+' '+word:word;
    if(font.widthOfTextAtSize(line,size)<=width){current=line;continue;}
    flush();
    if(font.widthOfTextAtSize(word,size)<=width){current=word;continue;}
    for(const char of word){
      if(current&&font.widthOfTextAtSize(current+char,size)>width)flush();
      current+=char;
    }
  }
  flush();
  return result;
}

export async function createNotePdf(markdown:string,title='Compte rendu'):Promise<Blob>{
  const pdf=await PDFDocument.create();
  pdf.setTitle(title);pdf.setCreator('ScribeWatch');
  const normal=await pdf.embedFont(StandardFonts.Helvetica);
  const strong=await pdf.embedFont(StandardFonts.HelveticaBold);
  const pages:PDFPage[]=[];let page!:PDFPage;let y=0;
  const newPage=()=>{page=pdf.addPage([W,H]);pages.push(page);y=H-M;};
  const need=(h:number)=>{if(y-h<M+24)newPage();};
  newPage();
  const heading=printable(title,strong).slice(0,70)||'Compte rendu';
  page.drawText(heading,{x:M,y:y-16,font:strong,size:16,color:rgb(.16,.18,.20)});
  y-=43;
  const parsed=splitEditable(markdown);
  const source=(parsed.section?parsed.body:markdown)
    .replace(/^---\s*\r?\n[\s\S]*?\r?\n---\s*\r?\n/,'');
  for(const line of source.split(/\r?\n/)){
    const entry=line.trim();
    if(!entry){y-=6;continue;}
    if(/^---+$/.test(entry)||/^\x60{3}/.test(entry))continue;
    const header=entry.match(/^(#{1,5})\s+(.+)$/);
    const isList=/^(?:[-*+]|\d+[.)])\s+/.test(entry);
    const font=header?strong:normal;
    const size=header?(header[1].length<=2?12:11):9.6;
    const indent=isList?12:0;
    const text=printable(header?header[2]:(isList?entry.replace(/^(?:[-*+]|\d+[.)])\s+/,'- '):entry),font);
    if(!text)continue;
    const rows=wrap(text,font,size,W-2*M-indent);
    need(header?27:14);
    if(header)y-=10;
    for(const row of rows){
      need(header?17:14);
      page.drawText(row,{x:M+indent,y,font,size,color:rgb(.16,.18,.20)});
      y-=header?17:14;
    }
    y-=header?4:1;
  }
  pages.forEach((pg,index)=>pg.drawText(String(index+1)+' / '+String(pages.length),{
    x:W-M-33,y:30,font:normal,size:8,color:rgb(.44,.46,.49)
  }));
  const bytes=await pdf.save();
  return new Blob([new Uint8Array(bytes).buffer],{type:'application/pdf'});
}

export async function buildSavedNotePdf(report:SavedReport):Promise<{blob:Blob;filename:string}>{
  const blob=await createNotePdf(report.markdown);
  const id=report.jobId.replace(/[^a-zA-Z0-9]/g,'').slice(0,8)||'document';
  const revision=report.revision>0?'_v'+report.revision:'';
  return {blob,filename:'compte-rendu_'+id+revision+'.pdf'};
}
