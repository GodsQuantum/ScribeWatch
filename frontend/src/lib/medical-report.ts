/** Client-side medical PDF: no patient data sent to another PDF service. */
import { PDFDocument, StandardFonts, rgb, degrees, type PDFFont, type PDFPage } from 'pdf-lib';
import {uniqueDirectoryFilename} from './local-save';
export interface PatientMetadata {
  surname:string;forename:string;dateOfBirth:string;
  practitioner?:string;practiceAddress?:string;practiceContact?:string;
  sourceSessionRef?:string;
  sourceDocumentSha256?:string;
  structuredProfile?:string;
  structuredModel?:string;
  sessionFields?:Array<{label:string;value:string}>;
}
export function medicalFilename(patient:PatientMetadata,date=new Date()):string{
  const letters=(v:string)=>v.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toUpperCase().replace(/[^A-Z]/g,'').slice(0,3);
  const a=letters(patient.surname),b=letters(patient.forename);
  const day=[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-');
  return (a&&b?a+b+'_':'CONSULTATION_')+day+'.pdf';
}
/** A saved consultation report must not overwrite another encounter on the same day. */
export async function uniqueMedicalFilename(directory:FileSystemDirectoryHandle,filename:string):Promise<string>{
  return uniqueDirectoryFilename(directory,filename);
}

/** Suppress internal review/audit metadata from patient-facing reports.
 * The original source and findings are preserved privately by the backend.
 * Never remove an ordinary clinical "à confirmer" or a patient follow-up.
 */
export function patientFacingMedicalBody(body:string):string{
  const internal=/^#{1,6}\s+(?:Éléments? à vérifier par le praticien|À confirmer par le praticien|Références documentaires(?:\s*\([^)]*\))?|Contrôle interne(?: de qualité)?|Points? . (?:confirmer|verifier|valider)|Audit clinique interne|Questions? et incertitudes à confirmer|Incertitudes? à (?:confirmer|vérifier)|Uncertainties or items to verify)\s*$/i;
  // Defence in depth: PDF header receives identity from the user-controlled
  // form. Do not carry contact coordinates hallucinated by a local model
  // over into clinical paragraphs, even when the backend is bypassed.
  const contact=/\b\d{1,4}\s+(?:rue|avenue|boulevard|impasse|allée|allee|route|chemin)\b|[\w.+-]+@[\w.-]+\.[a-z]{2,}|\b0[1-9](?:[ .-]*\d{2}){4}\b/i;
  let suppress=false;
  const kept:string[]=[];
  for(const line of body.split('\n')){
    const trimmed=line.trim();
    if(internal.test(trimmed)){suppress=true;continue;}
    // Long-source 'Partie N' is internal segmentation, not a clinical theme.
    // Remove only the literal generated heading; preserve every clinical line.
    if(/^##\s+Partie\s+\d+\s*[—–-]\s*déroulement de la consultation\s*$/i.test(trimmed)){
      if(kept.at(-1)?.trim()==='')kept.pop();
      continue;
    }
    if(suppress && /^#{1,6}\s+/.test(trimmed))suppress=false;
    if(suppress)continue;
    if(contact.test(trimmed))continue;
    if(/(?:formulation clinique écartée|information écartée\s*:|passage\(s\) à risque ont été écartés)/i.test(trimmed))continue;
    if(/^(?:ce document|ce compte rendu|cette synthèse).{0,260}(?:fidèlement|finalisation|validé par le praticien)/i.test(trimmed))continue;
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
  if(/^#{1,6}\s+/.test(cleaned) && !/^#{1,6}\s+Transcript\b/im.test(cleaned))
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

export async function createMedicalPdf(markdown:string,patient:PatientMetadata,date=new Date(),compactLastPage=false,denseShortLists=false):Promise<Blob>{
  const pdf=await PDFDocument.create();
  pdf.setTitle('Compte rendu de consultation');pdf.setCreator('ScribeWatch');
  // Source IDs, model names and checksums must never appear in patient PDFs.
  const regular=await pdf.embedFont(StandardFonts.Helvetica);
  const bold=await pdf.embedFont(StandardFonts.HelveticaBold);
  const W=595.28,H=841.89,M=denseShortLists?42:compactLastPage?44:49,contentWidth=W-2*M;
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
  const draw=(text:string,style:'title'|'section'|'subsection'|'body'|'bullet'|'table'='body')=>{
    const size=style==='title'?17:style==='section'?(denseShortLists?11.4:compactLastPage?11.6:12):style==='subsection'?(denseShortLists?10.4:compactLastPage?10.6:10.8):(denseShortLists?10:compactLastPage?10.2:10.3);
    const font=style==='body'||style==='bullet'?regular:bold;
    const marker=style==='section'?sectionIcon(text):null;
    const indent=(style==='bullet'?12:0)+(marker?18:0),leading=size*(denseShortLists?1.23:compactLastPage?1.26:1.37);
    const rich=(style==='body'||style==='bullet')?wrapRichText(text,regular,bold,size,contentWidth-indent):null;
    const plain=rich?null:wrap(text,font,size,contentWidth-indent);
    if(!(rich?.length||plain?.length))return;
    // Section and subsection headings must stay with at least two text lines.
    // Avoid a heading being orphaned at the foot of an A4 page.
    ensure((style==='section'||style==='subsection')?leading*4+18:leading*2+8);
    if(style==='title'||style==='section'){y-=compactLastPage?5:9;ensure(16);y-=compactLastPage?4:7;}
    else if(style==='subsection'){y-=compactLastPage?3:4;ensure(12);y-=compactLastPage?3:4;}
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
    y-=style==='title'||style==='section'?(denseShortLists?4:compactLastPage?5:9):style==='subsection'?(denseShortLists?3:compactLastPage?4:6):(denseShortLists?1:compactLastPage?2:4);
  };
  // Preflight with the same font, wrap and line-spacing metrics used when
  // actually drawing. Move a short section/subsection together, but only
  // when it can fit on an otherwise empty page; longer sections may flow.
  const availableOnFreshPage=H-58-67;
  type MedicalBlock =
    | {kind:'space'}
    | {kind:'text'; text:string; style:'section'|'subsection'|'body'|'bullet';level?:number}
    | {kind:'table'; rows:string[][]};
  const estimateText=(text:string,style:'section'|'subsection'|'body'|'bullet')=>{
    const size=style==='section'?(denseShortLists?11.4:compactLastPage?11.6:12):style==='subsection'?(denseShortLists?10.4:compactLastPage?10.6:10.8):(denseShortLists?10:compactLastPage?10.2:10.3);
    const font=style==='body'||style==='bullet'?regular:bold;
    const indent=(style==='bullet'?12:0)+(style==='section'&&sectionIcon(text)?18:0);
    const count=(style==='body'||style==='bullet'
      ?wrapRichText(text,regular,bold,size,contentWidth-indent).length
      :wrap(text,font,size,contentWidth-indent).length);
    if(!count)return 0;
    const leading=size*(denseShortLists?1.23:compactLastPage?1.26:1.37);
    const before=style==='section'?(denseShortLists?6:compactLastPage?9:16):style==='subsection'?(denseShortLists?4:compactLastPage?6:8):0;
    const after=style==='section'?(denseShortLists?4:compactLastPage?5:9):style==='subsection'?(denseShortLists?3:compactLastPage?4:6):(denseShortLists?1:compactLastPage?2:4);
    return before+count*leading+after;
  };
  const tableRowHeight=(cells:string[],columns:number,headline:boolean)=>{
    const font=headline?bold:regular,cellWidth=contentWidth/columns;
    const rows=Array.from({length:columns},(_,i)=>wrap(cells[i]||'',font,9,cellWidth-16));
    return Math.max(18,...rows.map(row=>row.length*12.6))+16;
  };
  const estimateTable=(rows:string[][])=>{
    if(!rows.length)return 0;
    const columns=rows[0].length;
    if(columns<2||columns>4)return rows.reduce((n,r)=>n+estimateText(r.join(' : '),'body'),0);
    return rows.reduce((n,row,i)=>n+tableRowHeight(row,columns,i===0),15);
  };
  const estimate=(block:MedicalBlock):number=>{
    if(block.kind==='space')return 3;
    if(block.kind==='table')return estimateTable(block.rows);
    return estimateText(block.text,block.style);
  };
  const drawTable=(rows:string[][])=>{
    if(!rows.length)return;
    const columns=rows[0].length;
    if(columns<2||columns>4){
      rows.forEach(row=>draw(row.join(' : '),'body'));return;
    }
    const cellWidth=contentWidth/columns;
    const ink=rgb(.23,.29,.32),light=rgb(.94,.97,.97),edge=rgb(.72,.80,.82);
    const drawRow=(cells:string[],headline:boolean)=>{
      const font=headline?bold:regular,size=9.0,leading=12.6,padding=8;
      const wrapped=Array.from({length:columns},(_,i)=>wrap(cells[i]||'',font,size,cellWidth-2*padding));
      const rowHeight=tableRowHeight(cells,columns,headline);
      if(rowHeight>availableOnFreshPage){
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
    // Keep short tables intact; for oversized tables repeat the header on each
    // page and never split individual rows.
    if(estimateTable(rows)<=availableOnFreshPage && y-estimateTable(rows)<67)addPage();
    const header=rows[0];
    for(let i=0;i<rows.length;i++){
      const rowHeight=tableRowHeight(rows[i],columns,i===0);
      const need=rowHeight+(i>0?tableRowHeight(header,columns,true):0);
      if(i>0&&y-need<67){
        addPage();
        drawRow(header,true);
      }
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
  const blocks:MedicalBlock[]=[];
  // Generic user-entered session metadata: deterministic, no model inference.
  // Render with the same pagination grouping as all other document sections.
  const metadata=(patient.sessionFields??[]).slice(0,24)
    .map(field=>({label:(field.label||'').trim().slice(0,90),
      value:(field.value||'').trim().slice(0,500)}))
    .filter(field=>field.label&&field.value);
  if(metadata.length){
    blocks.push({kind:'text',text:'Informations de séance',style:'section',level:2});
    metadata.forEach(field=>blocks.push({
      kind:'text',text:field.label+' : '+field.value,style:'body'
    }));
    blocks.push({kind:'space'});
  }
  for(let i=0;i<lines.length;i++){
    const line=lines[i].trim();
    if(!line){blocks.push({kind:'space'});continue;}
    if(line.startsWith('|')){
      const group:string[]=[];
      while(i<lines.length&&lines[i].trim().startsWith('|'))group.push(lines[i++].trim());
      i--;
      const rows=group.filter(item=>!/^[|\s:\-]+$/.test(item)).map(item=>{
        const bounded=item.replace(/^\|/,'').replace(/\|$/,'');
        return bounded.split('|').map(value=>value.trim());
      });
      if(rows.length)blocks.push({kind:'table',rows});
    }else if(/^#{1,6}\s/.test(line)){
      const level=line.match(/^#{1,6}/)?.[0].length||2;
      blocks.push({kind:'text',text:line.replace(/^#{1,6}\s+/,''),style:level>=3?'subsection':'section',level});
    }else if(/^[-*]\s/.test(line)||/^\d+[.)]\s/.test(line)){
      blocks.push({kind:'text',text:'• '+line.replace(/^(?:[-*]|\d+[.)])\s+/ ,''),style:'bullet'});
    }else blocks.push({kind:'text',text:line,style:'body'});
  }
  // Source-partitioned LLMs often emit dozens of one-line bullets. Group a
  // dense run into short, independently readable paragraphs instead of
  // spending one printed line per small fact. This is layout-only: every word
  // and every bullet marker remains unchanged, in source order.
  // Short lists and mixed headings/tables retain their original layout.
  const bulletCount=blocks.filter(b=>b.kind==='text'&&b.style==='bullet').length;
  if(denseShortLists){
    const grouped:MedicalBlock[]=[];
    for(let i=0;i<blocks.length;){
      const start=i;
      while(i<blocks.length){
        const candidate=blocks[i];
        if(candidate.kind!=='text'||candidate.style!=='bullet')break;
        i++;
      }
      const run=blocks.slice(start,i);
      if(run.length>=4){
        let merged='';
        let count=0;
        const flush=()=>{
          if(merged)grouped.push({kind:'text',text:merged,style:'body'});
          merged='';count=0;
        };
        for(const entry of run){
          if(entry.kind!=='text')continue;
          if(entry.text.length>155){flush();grouped.push(entry);continue;}
          if(merged&&(merged.length+entry.text.length+2>240||count>=3))flush();
          merged+=(merged?'  ':'')+entry.text;
          count++;
        }
        flush();
      }else grouped.push(...run);
      if(start===i){grouped.push(blocks[i]);i++;}
    }
    blocks.splice(0,blocks.length,...grouped);
  }
  const isHeading=(block:MedicalBlock):block is Extract<MedicalBlock,{kind:'text'}> =>
    block.kind==='text'&&(block.style==='section'||block.style==='subsection');
  for(let i=0;i<blocks.length;i++){
    const block=blocks[i];
    if(isHeading(block)){
      if(denseShortLists){
        // In a high-density report, keeping an entire heading section
        // together strands substantial white space on previous pages. Keep
        // only the heading plus its first readable lines, even when a section
        // itself is long. This does not omit or truncate a single block.
        let first=i+1;
        while(first<blocks.length&&blocks[first].kind==='space')first++;
        const following=blocks[first];
        let opening=estimate(block)+28;
        if(following?.kind==='text'){
          if(isHeading(following)){
            let next=first+1;
            while(next<blocks.length&&blocks[next].kind==='space')next++;
            opening+=estimate(following)+Math.min(
              next<blocks.length?estimate(blocks[next]):28,42
            );
          }else opening+=Math.min(estimate(following),42);
        }else if(following?.kind==='table')opening+=42;
        if(y-67<opening)addPage();
      }else{
      // Full H2/H3 block until the next heading of the same or higher rank.
      // H3 also prevents a long H2 from forcing an entire page per section.
      let height=0,firstContent=0;
      for(let j=i;j<blocks.length;j++){
        const next=blocks[j];
        if(j>i && isHeading(next) && (next.level??2)<=(block.level??2))break;
        const itemHeight=estimate(next);
        height+=itemHeight;
        if(!firstContent && j>i && next.kind!=='space')firstContent=itemHeight;
        if(height>availableOnFreshPage)break;
      }
      if(height<=availableOnFreshPage){
        const remaining=y-67;
        // A near-page-sized section should not leave a mostly empty sheet
        // behind it. Keep compact sections together, but let an almost-full
        // page of content flow when there is ample useful space left.
        const nearPageSized=height>availableOnFreshPage*0.55;
        const substantialSpaceLeft=remaining>availableOnFreshPage*0.36;
        if(remaining<height && !(nearPageSized && substantialSpaceLeft))addPage();
      }else{
        // Cannot keep a lengthy section on one page. At least keep the
        // heading plus a small opening portion with it, without wasting a
        // whole page to accommodate an impossible keep-together request.
        let opening=estimate(block)+Math.min(firstContent||32,42);
        let firstChildWouldMove=false;
        // A long parent section can begin with a short H3/H4 subsection.
        // Check that child's full footprint, not just the height of its
        // heading: otherwise the parent H2 can be orphaned at a page foot
        // while the first child jumps to the following page.
        {
          let childIndex=i+1;
          while(childIndex<blocks.length&&blocks[childIndex].kind==='space')childIndex++;
          const child=blocks[childIndex];
          if(child&&isHeading(child)&&(child.level??2)>(block.level??2)){
            let childHeight=0;
            for(let j=childIndex;j<blocks.length;j++){
              const next=blocks[j];
              if(j>childIndex&&isHeading(next)
                && (next.level??2)<=(child.level??2))break;
              childHeight+=estimate(next);
              if(childHeight>availableOnFreshPage)break;
            }
            opening=estimate(block)+(
              childHeight<=availableOnFreshPage-estimate(block)
                ?childHeight
                :estimate(child)+48
            );
            // Evaluate the child's actual keep-together rule at the
            // position AFTER its parent heading. If the child would start
            // on the next page, its parent must follow it, never orphaned.
            const remainingAfterParent=y-67-estimate(block);
            const childIsPageSized=childHeight>availableOnFreshPage*0.55;
            firstChildWouldMove=childHeight<=availableOnFreshPage
              && remainingAfterParent<childHeight
              && !(childIsPageSized&&remainingAfterParent>availableOnFreshPage*0.36);
          }
        }
        const left=y-67;
        const nearFullOpening=opening>availableOnFreshPage*0.55;
        if(firstChildWouldMove
          || (left<opening && !(nearFullOpening&&left>availableOnFreshPage*0.36)))
          addPage();
      }
      }
    }else if(block.kind==='table' && estimate(block)<=availableOnFreshPage){
      // Standalone table must not start as an orphan at a page foot.
      if(y-estimate(block)<67)addPage();
    }
    if(block.kind==='space')y-=denseShortLists?1:compactLastPage?2:3;
    else if(block.kind==='table')drawTable(block.rows);
    else draw(block.text,block.style);
  }
  // Compare readable standard, compact, and dense-short-list layouts. Choose
  // fewer pages without rewriting any clinical statement. Dense lists are
  // considered only when the original contains many bullets and saves a page;
  // ordinary short documents retain their existing generous typography.
  if(!compactLastPage && pages.length>1){
    const compact=await createMedicalPdf(markdown,patient,date,true);
    const compactPages=(await PDFDocument.load(await compact.arrayBuffer())).getPageCount();
    if(bulletCount>=12){
      const dense=await createMedicalPdf(markdown,patient,date,true,true);
      const densePages=(await PDFDocument.load(await dense.arrayBuffer())).getPageCount();
      if(densePages<Math.min(pages.length,compactPages))
        return dense;
    }
    if(compactPages<pages.length || (H-58-y)<150)
      return compact;
  }
  for(let i=0;i<pages.length;i++){
    const p=pages[i];
    p.drawLine({start:{x:M,y:53},end:{x:W-M,y:53},thickness:.7,color:rgb(.78,.83,.84)});
    const n='Page '+(i+1)+' / '+pages.length;
    p.drawText(n,{x:W-M-bold.widthOfTextAtSize(n,8),y:37,size:8,font:bold,color:muted});
  }
  return new Blob([new Uint8Array(await pdf.save())],{type:'application/pdf'});
}
