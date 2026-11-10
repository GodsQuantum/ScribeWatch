/** Client-side medical PDF: no patient data sent to another PDF service. */
import { PDFDocument, StandardFonts, rgb, degrees, type PDFFont, type PDFPage } from 'pdf-lib';
export interface PatientMetadata {
  surname:string;forename:string;dateOfBirth:string;
  practitioner?:string;practiceAddress?:string;practiceContact?:string;
  sourceSessionRef?:string;
  sourceDocumentSha256?:string;
  structuredProfile?:string;
  structuredModel?:string;
}
export function medicalFilename(patient:PatientMetadata,date=new Date()):string{
  const letters=(v:string)=>v.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toUpperCase().replace(/[^A-Z]/g,'').slice(0,3);
  const a=letters(patient.surname),b=letters(patient.forename);
  const day=[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-');
  return (a&&b?a+b+'_':'CONSULTATION_')+day+'.pdf';
}
/** A saved consultation report must not overwrite another encounter on the same day. */
export async function uniqueMedicalFilename(directory:FileSystemDirectoryHandle,filename:string):Promise<string>{
  const dot=filename.lastIndexOf('.');
  const base=dot>=0?filename.slice(0,dot):filename;
  const extension=dot>=0?filename.slice(dot):'';
  for(let n=1;n<=999;n++){
    const candidate=n===1?filename:base+'_'+n+extension;
    try{
      await directory.getFileHandle(candidate,{create:false});
    }catch(error){
      if(error instanceof DOMException&&error.name==='NotFoundError')return candidate;
      throw error;
    }
  }
  throw new Error('Too many reports with the same name in this directory');
}

/** Suppress internal review/audit metadata from patient-facing reports.
 * The original source and findings are preserved privately by the backend.
 * Never remove an ordinary clinical "à confirmer" or a patient follow-up.
 */
export function patientFacingMedicalBody(body:string):string{
  const internal=/^#{1,3}\s+(?:Éléments? à vérifier par le praticien|À confirmer par le praticien|Références documentaires(?:\s*\([^)]*\))?|Contrôle interne(?: de qualité)?|Audit clinique interne)\s*$/i;
  // Defence in depth: PDF header receives identity from the user-controlled
  // form. Do not carry contact coordinates hallucinated by a local model
  // over into clinical paragraphs, even when the backend is bypassed.
  const contact=/\b\d{1,4}\s+(?:rue|avenue|boulevard|impasse|allée|allee|route|chemin)\b|[\w.+-]+@[\w.-]+\.[a-z]{2,}|\b0[1-9](?:[ .-]*\d{2}){4}\b/i;
  let suppress=false;
  const kept:string[]=[];
  for(const line of body.split('\n')){
    const trimmed=line.trim();
    if(internal.test(trimmed)){suppress=true;continue;}
    if(suppress && /^#{1,3}\s+/.test(trimmed))suppress=false;
    if(suppress)continue;
    if(contact.test(trimmed))continue;
    if(/(?:formulation clinique écartée|information écartée\s*:|passage\(s\) à risque ont été écartés)/i.test(trimmed))continue;
    kept.push(line);
  }
  return kept.join('\n').trim();
}

export function structuredMedicalBody(text:string):string{
  const matched=text.match(/^## Structured notes\s*([\s\S]*?)(?=^## Transcript\s*$|(?![\s\S]))/m);
  if(matched?.[1]?.trim())return patientFacingMedicalBody(matched[1]);
  const cleaned=text.replace(/^---\s*\n[\s\S]*?\n---\s*\n/,'').trim();
  // A provider may return the completed clinical Markdown directly, without
  // the old "Structured notes" transport wrapper. Never relabel it as raw ASR.
  if(/^#{1,3}\s+/.test(cleaned) && !/^#{1,3}\s+Transcript\b/im.test(cleaned))
    return patientFacingMedicalBody(cleaned);
  return '## Transcription non structurée - à vérifier\n\n'+cleaned;
}
function safe(text:string,font:PDFFont):string {
  const plain=text.replace(/<br\s*\/?>/gi,' ; ').replace(/<[^>]+>/g,' ')
    .replace(/\*\*([^*]+)\*\*/g,'$1').replace(/__([^_]+)__/g,'$1')
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
type RichRun = {text:string; bold:boolean};
/** Wrap Markdown strong emphasis with real bold font metrics, not stripped text. */
export function wrapRichText(text:string,normal:PDFFont,strong:PDFFont,size:number,width:number):RichRun[][]{
  const rows:RichRun[][]=[[]];let used=0;
  const put=(value:string,isBold:boolean)=>{
    if(!value)return;
    const font=isBold?strong:normal,row=rows[rows.length-1],last=row[row.length-1];
    if(last?.bold===isBold)last.text+=value;
    else row.push({text:value,bold:isBold});
    used+=font.widthOfTextAtSize(value,size);
  };
  const newline=()=>{
    const row=rows[rows.length-1];
    while(row.length && /^\s+$/.test(row[row.length-1].text))row.pop();
    if(row.length)rows.push([]);
    used=0;
  };
  const append=(original:string,isBold:boolean)=>{
    // safe() normalizes content but trims spaces: restore segment boundaries
    // so **bold**. and **bold** words keep their actual punctuation/spacing.
    const lead=/^\s/.test(original),trail=/\s$/.test(original);
    const core=safe(original,normal);
    const cleaned=(lead?' ':'')+core+(trail?' ':'');
    const font=isBold?strong:normal;
    for(const token of cleaned.match(/\s+|\S+/g)||[]){
      if(/^\s+$/.test(token)){
        const last=rows[rows.length-1].at(-1);
        if(used>0 && !last?.text.endsWith(' '))put(' ',false);
        continue;
      }
      if(used && used+font.widthOfTextAtSize(token,size)>width)newline();
      if(font.widthOfTextAtSize(token,size)<=width){put(token,isBold);continue;}
      for(const char of token){
        if(used && used+font.widthOfTextAtSize(char,size)>width)newline();
        put(char,isBold);
      }
    }
  };
  for(const raw of text.split(/(\*\*[^*]+\*\*|__[^_]+__)/g).filter(Boolean)){
    const bold=(raw.startsWith('**')&&raw.endsWith('**'))||(raw.startsWith('__')&&raw.endsWith('__'));
    append(bold?raw.slice(2,-2):raw,bold);
  }
  for(const row of rows){
    while(row.length && /^\s+$/.test(row[row.length-1].text))row.pop();
  }
  return rows.filter(row=>row.length);
}

export async function createMedicalPdf(markdown:string,patient:PatientMetadata,date=new Date()):Promise<Blob>{
  const pdf=await PDFDocument.create();
  pdf.setTitle('Compte rendu de consultation');pdf.setCreator('ScribeWatch');
  // Source IDs, model names and checksums must never appear in patient PDFs.
  const regular=await pdf.embedFont(StandardFonts.Helvetica);
  const bold=await pdf.embedFont(StandardFonts.HelveticaBold);
  const W=595.28,H=841.89,M=49,contentWidth=W-2*M;
  // Neutral print letter: avoid decorative app branding on medical correspondence.
  const navy=rgb(.14,.16,.18),teal=rgb(.14,.16,.18),muted=rgb(.42,.46,.5);
  let page!:PDFPage;let y=0;const pages:PDFPage[]=[];
  const addPage=()=>{
    page=pdf.addPage([W,H]);pages.push(page);y=H-58;
  };
  const ensure=(height:number)=>{if(y-height<67)addPage();};
  const sectionIcon=(label:string):'tooth'|'calendar'|'pill'|null=>{
    if(/🦷|carie|gencive|dentaire|orthodont|sagesse|hygiène/i.test(label))return 'tooth';
    if(/📅|prochaines étapes|suivi|rendez-vous/i.test(label))return 'calendar';
    if(/💊|ordonnance|sommeil|médicament/i.test(label))return 'pill';
    return null;
  };
  const drawSectionIcon=(kind:'tooth'|'calendar'|'pill',baseline:number)=>{
    const x=M+1,cy=baseline+2;
    if(kind==='tooth'){
      page.drawSvgPath('M 4 2 C 1 2 0 6 2 11 C 3 14 4 20 7 20 C 9 20 8 14 11 14 C 14 14 13 20 16 20 C 19 20 19 14 20 11 C 22 6 20 2 17 2 C 14 2 13 5 11 5 C 9 5 7 2 4 2 Z',
        {x,y:cy+8,scale:.62,borderColor:navy,borderWidth:.9,color:rgb(.96,.97,.98)});
    }else if(kind==='calendar'){
      page.drawRectangle({x:x+1,y:cy-8,width:12,height:12,borderColor:navy,borderWidth:.9});
      page.drawLine({start:{x:x+1,y:cy},end:{x:x+13,y:cy},thickness:.8,color:navy});
      page.drawCircle({x:x+4,y:cy-3,size:.75,color:navy});
      page.drawCircle({x:x+9,y:cy-3,size:.75,color:navy});
    }else{
      page.drawEllipse({x:x+7,y:cy-2,xScale:7,yScale:4,rotate:degrees(-38),
        color:rgb(.96,.97,.98),borderColor:navy,borderWidth:.9});
      page.drawLine({start:{x:x+4,y:cy+1},end:{x:x+10,y:cy-5},thickness:.8,color:navy});
    }
  };
  const draw=(text:string,style:'title'|'section'|'body'|'bullet'|'table'='body')=>{
    const size=style==='title'?17:style==='section'?12:10.3;
    const font=style==='body'||style==='bullet'?regular:bold;
    const marker=style==='section'?sectionIcon(text):null;
    const indent=(style==='bullet'?12:0)+(marker?18:0),leading=size*1.43;
    const rich=(style==='body'||style==='bullet')?wrapRichText(text,regular,bold,size,contentWidth-indent):null;
    const plain=rich?null:wrap(text,font,size,contentWidth-indent);
    if(!(rich?.length||plain?.length))return;
    ensure(leading*2+8);
    if(style==='title'||style==='section'){y-=9;ensure(16);y-=7;}
    if(marker)drawSectionIcon(marker,y);
    for(let i=0;i<(rich?.length??plain?.length??0);i++){
      ensure(leading);
      if(rich){
        let x=M+indent;
        for(const run of rich[i]){
          const selected=run.bold?bold:regular;
          page.drawText(run.text,{x,y,size,font:selected,color:navy});
          x+=selected.widthOfTextAtSize(run.text,size);
        }
      }else if(plain){page.drawText(plain[i],{x:M+indent,y,size,font,color:teal});}
      y-=leading;
    }
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
  page.drawText('DATE DE CONSULTATION  /  '+safe(dateLabel,bold),{x:M,y,size:10,font:bold,color:navy});
  y-=30;
  let columnStart=y;
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
  const practiceLines=[
    patient.practitioner?.trim()||'Praticien à renseigner',
    patient.practiceAddress?.trim()||'',
    patient.practiceContact?.trim()||''
  ];
  const patientName=[patient.forename.trim(),patient.surname.trim().toLocaleUpperCase('fr-FR')].filter(Boolean).join(' ')||'Identité à renseigner';
  const patientLines=[
    patientName,
    'Date de naissance : '+(patient.dateOfBirth
      ?(patient.dateOfBirth.match(/^\d{4}-\d{2}-\d{2}$/)
        ?patient.dateOfBirth.split('-').reverse().join('/'):patient.dateOfBirth)
      :'à renseigner')
  ];
  // Size the metadata panel for long addresses: no clipping or text overlap.
  const lineCount=(lines:string[],width:number)=>lines.filter(Boolean)
    .reduce((sum,line)=>sum+wrap(line,regular,10,width).length,0);
  const panelHeight=Math.max(94,45+14*Math.max(lineCount(practiceLines,210),lineCount(patientLines,200)));
  ensure(panelHeight+25);
  columnStart=y;
  // Two plain columns as on a professional French consultation letter.
  const left=drawColumn(M+5,'PRATICIEN',practiceLines,210);
  const right=drawColumn(M+285,'PATIENT',patientLines,200);
  y=Math.min(left,right)-15;
  page.drawLine({start:{x:M,y},end:{x:W-M,y},thickness:.8,color:rgb(.77,.82,.84)});
  y-=26;
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
    const n='Page '+(i+1)+' / '+pages.length;
    p.drawText(n,{x:W-M-bold.widthOfTextAtSize(n,8),y:37,size:8,font:bold,color:muted});
  }
  return new Blob([new Uint8Array(await pdf.save())],{type:'application/pdf'});
}
