/** Client-side medical PDF: no patient data sent to another PDF service. */
import { PDFDocument, StandardFonts, rgb, type PDFFont, type PDFPage } from 'pdf-lib';
export interface PatientMetadata {
  surname:string;forename:string;dateOfBirth:string;
  practitioner?:string;practiceAddress?:string;practiceContact?:string;
}
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
function safe(text:string,font:PDFFont):string {
  const plain=text.replace(/\*\*([^*]+)\*\*/g,'$1').replace(/__([^_]+)__/g,'$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g,'$1').replace(/[`]/g,'');
  let out='';
  for(const char of plain){
    try{font.encodeText(char);out+=char;}
    catch{out+=(char.codePointAt(0)??0)>0xFFFF?'':'?';}
  }
  return out.replace(/\s+/g,' ').trim();
}

function wrap(text:string,font:PDFFont,size:number,width:number):string[]{
  const lines:string[]=[];let line='';
  for(const word of safe(text,font).split(/\s+/).filter(Boolean)){
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
  let page!:PDFPage;let y=0;const pages:PDFPage[]=[];
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
  const drawTable=(rows:string[][])=>{
    if(!rows.length)return;
    const columns=rows[0].length;
    if(columns<2||columns>4){
      rows.forEach(row=>draw(row.join(' : '),'body'));return;
    }
    const cellWidth=contentWidth/columns;
    const ink=rgb(.23,.29,.32),light=rgb(.94,.97,.97),edge=rgb(.72,.80,.82);
    let header: string[]|undefined=rows[0];
    const drawRow=(cells:string[],headline:boolean)=>{
      const font=headline?bold:regular,size=9.0,leading=12.6,padding=8;
      const wrapped=Array.from({length:columns},(_,i)=>wrap(cells[i]||'',font,size,cellWidth-2*padding));
      const rowHeight=Math.max(18,...wrapped.map(row=>row.length*leading))+2*padding;
      if(rowHeight>H-170){
        cells.forEach((cell,index)=>draw('Colonne '+(index+1)+' : '+cell,'body'));return;
      }
      ensure(rowHeight+4);
      const bottom=y-rowHeight;
      page.drawRectangle({x:M,y:bottom,width:contentWidth,height:rowHeight,color:headline?light:rgb(1,1,1),borderColor:edge,borderWidth:.6});
      for(let i=0;i<columns;i++){
        if(i)page.drawLine({start:{x:M+i*cellWidth,y:bottom},end:{x:M+i*cellWidth,y},thickness:.6,color:edge});
        wrapped[i].forEach((line,index)=>page.drawText(line,{x:M+i*cellWidth+padding,y:y-padding-size-index*leading,size,font,color:headline?teal:ink}));
      }
      y=bottom;
    };
    for(let i=0;i<rows.length;i++){
      if(y-60<70){addPage();if(i>0&&header)drawRow(header,true);}
      drawRow(rows[i],i===0);
    }
    y-=15;
  };
  addPage();
  const dateLabel=new Intl.DateTimeFormat('fr-FR',{dateStyle:'long'}).format(date);
  page.drawText('Date : '+safe(dateLabel,bold),{x:M,y,size:10,font:bold,color:navy});
  y-=34;
  const columnStart=y;
  const drawColumn=(x:number,label:string,lines:string[],width:number):number=>{
    if(!lines.some(Boolean))return columnStart;
    let at=columnStart;
    page.drawText(label,{x,y:at,size:11,font:bold,color:teal});at-=21;
    for(const paragraph of lines.filter(Boolean)){
      for(const line of wrap(paragraph,regular,10,width)){
        page.drawText(line,{x,y:at,size:10,font:regular,color:navy});
        at-=14;
      }
    }
    return at;
  };
  const left=drawColumn(M,'PRATICIEN',[
    patient.practitioner?.trim()||'',
    patient.practiceAddress?.trim()||'',
    patient.practiceContact?.trim()||''
  ],220);
  const right=drawColumn(M+285,'PATIENT',[
    [patient.forename.trim(),patient.surname.trim().toLocaleUpperCase('fr-FR')].filter(Boolean).join(' '),
    patient.dateOfBirth?'Date de naissance : '+(patient.dateOfBirth.match(/^\d{4}-\d{2}-\d{2}$/)?patient.dateOfBirth.split('-').reverse().join('/'):patient.dateOfBirth):''
  ],contentWidth-285);
  y=Math.min(left,right)-14;
  page.drawLine({start:{x:M,y},end:{x:W-M,y},thickness:.8,color:rgb(.77,.82,.84)});
  y-=26;
  draw('Compte rendu de consultation','title');

  const body=structuredMedicalBody(markdown);
  const lines=body.split('\n');
  for(let i=0;i<lines.length;i++){
    const line=lines[i].trim();if(!line){y-=3;continue;}
    if(line.startsWith('|')){
      const group:string[]=[];
      while(i<lines.length&&lines[i].trim().startsWith('|'))group.push(lines[i++].trim());
      i--;
      const rows=group.filter(item=>!/^[|\s:\-]+$/.test(item)).map(item=>{
        const bounded=item.replace(/^\|/,'').replace(/\|$/,'');
        return bounded.split('|').map(value=>value.trim());
      });
      if(rows.length)drawTable(rows);
    }else if(/^#{1,3}\s/.test(line))draw(line.replace(/^#{1,3}\s+/,''),'section');
    else if(/^[-*]\s/.test(line)||/^\d+[.)]\s/.test(line))draw('• '+line.replace(/^(?:[-*]|\d+[.)])\s+/,''),'bullet');
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
