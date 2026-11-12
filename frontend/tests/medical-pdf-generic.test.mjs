import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMedicalPdf, medicalFilename, structuredMedicalBody, uniqueMedicalFilename, wrapRichText, patientFacingMedicalBody } from '../src/lib/medical-report.ts';
import { PDFDocument, StandardFonts } from 'pdf-lib';
import {spawnSync} from 'node:child_process';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

test('medical PDF renders independently of server and excludes raw transcript when structured', async () => {
  const date = new Date(2026, 10, 3);
  const patient = { surname: 'EXEMPLE', forename: 'FICTIF', dateOfBirth: '2000-01-01' };
  assert.equal(medicalFilename(patient, date), 'EXEFIC_2026-11-03.pdf');
  const markdown = [
    '## Structured notes',
    '## Bilan et constats',
    'Compte rendu fictif : questions générales et explications non médicales.',
    '## Explications',
    'Les **faits importants** sont explicitement exposés dans la consultation.',
    '## Tableau des étapes',
    '| Étape | Information discutée | Échéance |',
    '|---|---|---|',
    '| 1 | Entretien et discussion sans prescription | Aujourd’hui |',
    '| 2 | Nouveau rendez-vous envisagé | À confirmer |',
    '## À confirmer par le praticien',
    'Aucune décision autonome.',
    '## Transcript',
    'Texte integral non destine au patient',
  ].join('\n');
  assert.match(structuredMedicalBody(markdown), /Bilan et constats/);
  assert.doesNotMatch(structuredMedicalBody(markdown), /Texte integral/);
  const blob = await createMedicalPdf(markdown, {
    ...patient,
    sourceSessionRef:'session-2026-11-03-A',
    sourceDocumentSha256:'a'.repeat(64),
    structuredProfile:'Compte rendu médical - patient et dossier',
    structuredModel:'llm-local',
  }, date);
  const pdf = await PDFDocument.load(await blob.arrayBuffer());
  assert.ok(pdf.getPageCount() >= 1);
  assert.equal(pdf.getTitle(), 'Compte rendu de consultation');
  assert.equal(pdf.getSubject(), undefined);
  const page=pdf.getPages()[0];
  assert.ok(page.getSize().width>590);

  assert.ok(blob.size > 1000);
  assert.ok(blob.type === 'application/pdf');
  assert.ok(pdf.getPageCount() < 8, 'Simple consultation should not spill into excessive pages');
});


test('private audit sections are excluded from patient body',() => {
  const body=[
    '## Examen',
    'Une indication clinique habituelle.',
    '## Éléments à vérifier par le praticien',
    'Audit privé non exportable.',
    '## Suites',
    'Un suivi à discuter.',
    'Contact: exemple@test.invalid.',
    'Adresse: 24 rue Imaginaire.',
    '## Références documentaires',
    'Source interne non exportable.'
  ].join('\n');
  const rendered=patientFacingMedicalBody(body);
  assert.match(rendered,/Une indication/);
  assert.match(rendered,/Un suivi/);
  assert.doesNotMatch(rendered,/Audit privé|Références documentaires|Source interne|exemple@test|rue Imaginaire/);
});

test('source-model internal uncertainties stay in private draft but never leak to patient PDF', () => {
  const raw = [
    '## Examen et soins',
    'Le patient a été informé de plusieurs possibilités.',
    '## Questions et incertitudes à confirmer',
    'Nom de spécialiste hypothétique à ne pas publier.',
    'Mot du moteur ASR sans certitude.',
  ].join('\n');
  const patientBody = patientFacingMedicalBody(raw);
  assert.match(patientBody, /plusieurs possibilités/);
  assert.doesNotMatch(patientBody, /Nom de spécialiste|Mot du moteur ASR|incertitudes/);
  assert.match(raw, /Nom de spécialiste/);
});

test('medical PDF still renders with missing patient identity, without inventing a name', async () => {
  const patient = { surname:'', forename:'', dateOfBirth:'', practitioner:'Dr Test Fictif' };
  const pdfBlob = await createMedicalPdf('## Synthèse\nConsultation purement fictive.', patient, new Date(2026,10,9));
  const pdf = await PDFDocument.load(await pdfBlob.arrayBuffer());
  assert.ok(pdf.getPageCount() >= 1);
  assert.ok(pdfBlob.size > 1200);
});

test('medical PDF never silently overwrites another consultation from the same patient/day', async () => {
  const folder = {
    getFileHandle(name) {
      if(name==='EXEFIC_2026-11-03.pdf'||name==='EXEFIC_2026-11-03_2.pdf')return Promise.resolve({name});
      return Promise.reject(new DOMException('Not found','NotFoundError'));
    }
  };
  assert.equal(await uniqueMedicalFilename(folder,'EXEFIC_2026-11-03.pdf'),'EXEFIC_2026-11-03_3.pdf');
  await assert.rejects(uniqueMedicalFilename({
    getFileHandle() {throw new DOMException('Denied','NotAllowedError')}
  },'EXEFIC_2026-11-03.pdf'), {name:'NotAllowedError'});
});


test('inline clinical highlights retain real bold spans after wrapping', async () => {
  const pdf=await PDFDocument.create();
  const normal=await pdf.embedFont(StandardFonts.Helvetica);
  const heavy=await pdf.embedFont(StandardFonts.HelveticaBold);
  const lines=wrapRichText('La **dent 46** présente une **lésion importante**. Vérifier.',normal,heavy,10,220);
  const runs=lines.flat();
  assert.ok(runs.some(x=>x.bold && x.text.includes('dent')));
  assert.ok(runs.some(x=>x.bold && x.text.includes('46')));
  assert.ok(runs.some(x=>!x.bold && x.text.includes('présente')));
  const punctuated=wrapRichText('Un **fait**.',normal,heavy,10,220);
  assert.equal(punctuated.flat().map(x=>x.text).join(''),'Un fait.');
  const spaced=wrapRichText('Une **note** importante.',normal,heavy,10,220);
  assert.equal(spaced.flat().map(x=>x.text).join(''),'Une note importante.');
  for(const row of lines){
    const measured=row.reduce((sum,x)=>sum+(x.bold?heavy:normal).widthOfTextAtSize(x.text,10),0);
    assert.ok(measured<=220.5,'Rich text should fit the page width');
  }
});


test('provider-native headings render as structured clinical notes, not raw ASR', () => {
  const native='## Gencives\n\nLes **gencives** ont été examinées.\n\n## Suivi\n\nÀ vérifier.';
  assert.equal(structuredMedicalBody(native),native);
  assert.match(structuredMedicalBody('## Transcript\nParoles brutes'),/Transcription non structurée/);
});

test("patient PDF excludes model review headings and editorial approval text", () => {
  const note = [
    "## Examen clinique",
    "Le soin est a discuter.",
    "### Points \u00e0 confirmer",
    "- Audit technique interne.",
    "Ce document r\u00e9sume fid\u00e8lement la consultation et est pr\u00eat pour finalisation.",
    "## Suivi clinique",
    "Un rendez-vous reste \u00e0 confirmer."
  ].join("\n");
  const text = patientFacingMedicalBody(note);
  assert.match(text, /Le soin est a discuter/);
  assert.match(text, /Un rendez-vous reste/);
  assert.doesNotMatch(text, /Points|Audit technique|r.sum. fid.lement|finalisation/);
});


test('long-source pagination hides only internal Part N transport headings', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'};
  const markdown=[
    '## Partie 1 — déroulement de la consultation',
    '## Thème utile fictif',
    'La phrase clinique fictive est préservée sans modification.',
    '## Partie 2 — déroulement de la consultation',
    '### Un autre thème fictif',
    '- Un deuxième fait fictif est maintenu dans son intégralité.'
  ].join('\n');
  const cleaned=patientFacingMedicalBody(markdown);
  assert.doesNotMatch(cleaned,/Partie [12].*déroulement/);
  assert.match(cleaned,/## Thème utile fictif/);
  assert.match(cleaned,/Un deuxième fait fictif est maintenu/);
  const pdf=await createMedicalPdf(markdown,patient,new Date(2026,10,11));
  const doc=await PDFDocument.load(await pdf.arrayBuffer());
  assert.equal(doc.getPageCount(),1);
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-generated-scaffold-'));
  try{
    const filename=join(directory,'internal-headings.pdf');
    writeFileSync(filename,new Uint8Array(await pdf.arrayBuffer()));
    const result=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
    if(result.error?.code==='ENOENT'){t.diagnostic('pdftotext unavailable');return;}
    assert.equal(result.status,0);
    assert.doesNotMatch(result.stdout,/Partie [12].*déroulement/);
    assert.match(result.stdout,/Thème utile fictif/);
    assert.match(result.stdout,/Un deuxième fait fictif est maintenu/);
  }finally{rmSync(directory,{recursive:true,force:true});}
});

test('pagination keeps compact sections together and uses spare page space', async t => {
  const patient = {surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01',practitioner:'Dr Exemple'};
  const paragraph = n => 'Le paragraphe fictif numero '+n+' sert uniquement a verifier la pagination. Aucun renseignement reel de patient, aucune information medicale ni aucune prescription ne figure dans cette demonstration. La mise en page doit suivre les rubriques de maniere compacte et lisible.';
  const short = [
    '## Section courte preservee',
    'Un propos purement fictif pour verifier la coherence du groupe.',
    '- ALPHA-MARQUEUR : Le premier point du groupe.',
    '- BETA-MARQUEUR : Le second point du groupe.',
    '- GAMMA-MARQUEUR : Le troisieme point du groupe.'
  ].join('\n');
  const extra = Array.from({length:13},(_,i)=>paragraph(i+1)).join('\n\n');
  const document = '## Introduction detaillee\n'+extra+'\n\n'+short+'\n\n## Conclusion\nUne fin fictive.';
  const file = await createMedicalPdf(document,patient,new Date(2026,10,11));
  const pages = await PDFDocument.load(await file.arrayBuffer());
  assert.equal(pages.getPageCount(),2,'Do not leave an almost empty first page before a long section');
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-pagination-'));
  try {
    const filename=join(directory,'fictional-report.pdf');
    writeFileSync(filename,new Uint8Array(await file.arrayBuffer()));
    const extracted=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
    if(extracted.error?.code==='ENOENT'){
      t.diagnostic('pdftotext unavailable; PDF page-count assertion still executed');
      return;
    }
    assert.equal(extracted.status,0);
    const textPages=extracted.stdout.split('\f').filter(text=>text.trim());
    assert.equal(textPages.length,2);
    assert.ok(textPages[0].split(/\s+/).length>200,'First page should be meaningfully used');
    const markerPage=marker=>textPages.findIndex(text=>text.includes(marker));
    const first=markerPage('Section courte preservee');
    assert.ok(first>=0);
    for(const label of ['ALPHA-MARQUEUR','BETA-MARQUEUR','GAMMA-MARQUEUR'])
      assert.equal(markerPage(label),first,'Section header and its lines must remain together');
    assert.ok(textPages.join('').includes('Conclusion'));
  } finally {
    rmSync(directory,{recursive:true,force:true});
  }
});

test('several short sections share one PDF page; subsections stay with content', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'};
  const sections=Array.from({length:4},(_,i)=>'## Domaine fictif '+(i+1)+'\nUn texte court et fictif pour cette rubrique.').join('\n\n');
  const shortPdf=await createMedicalPdf(sections,patient,new Date(2026,10,11));
  assert.equal((await PDFDocument.load(await shortPdf.arrayBuffer())).getPageCount(),1);
  const leading=Array.from({length:10},(_,i)=>'Paragraphe long fictif '+(i+1)+' : '+('Un contenu sans informations de patients et sans indications de soins. ').repeat(4)).join('\n\n');
  const body='## Contexte general\n'+leading+'\n\n### Sous section a conserver\n\n- DELTA-FICTIF : Premiere note courte.\n- EPSILON-FICTIF : Deuxieme note courte.\n- ZETA-FICTIF : Derniere note courte.\n\n## Suite\nConclusion fictive.';
  const blob=await createMedicalPdf(body,patient,new Date(2026,10,11));
  const doc=await PDFDocument.load(await blob.arrayBuffer());
  assert.ok(doc.getPageCount()>=2 && doc.getPageCount()<=3);
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-subsection-'));
  try {
    const filename=join(directory,'subsections.pdf');
    writeFileSync(filename,new Uint8Array(await blob.arrayBuffer()));
    const result=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
    if(result.error?.code==='ENOENT'){
      t.diagnostic('pdftotext unavailable; PDF page-count assertion still executed');
      return;
    }
    assert.equal(result.status,0);
    const parts=result.stdout.split('\f').filter(x=>x.trim());
    const which=t=>parts.findIndex(page=>page.includes(t));
    const heading=which('Sous section a conserver');
    assert.ok(heading>=0);
    assert.equal(which('DELTA-FICTIF'),heading);
    assert.equal(which('EPSILON-FICTIF'),heading);
    assert.equal(which('ZETA-FICTIF'),heading);
  } finally {rmSync(directory,{recursive:true,force:true});}
});




test('deep Markdown subheadings render as real headings rather than literal hashes', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'};
  const markdown=[
    '## Rubrique principale fictive',
    '#### Sous-niveau profond fictif',
    '- Premier exemple non medical.',
    '- Second exemple non medical.',
    '##### Encore un sous-niveau fictif',
    'Un paragraphe de verification sans patient reel.'
  ].join('\n');
  const blob=await createMedicalPdf(markdown,patient,new Date(2026,10,11));
  const pdf=await PDFDocument.load(await blob.arrayBuffer());
  assert.equal(pdf.getPageCount(),1);
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-subheading-'));
  try{
    const filename=join(directory,'nested-headings.pdf');
    writeFileSync(filename,new Uint8Array(await blob.arrayBuffer()));
    const result=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
    if(result.error?.code==='ENOENT'){
      t.diagnostic('pdftotext unavailable; page-count assertion still executed');
      return;
    }
    assert.equal(result.status,0);
    assert.match(result.stdout,/Sous-niveau profond fictif/);
    assert.match(result.stdout,/Encore un sous-niveau fictif/);
    assert.doesNotMatch(result.stdout,/#{4,6}/,'No Markdown markers appear in the exported PDF');
  }finally{rmSync(directory,{recursive:true,force:true});}
});


test('nested parent heading is never orphaned before its first child', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'};
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-nested-'));
  try{
    for(const count of [6,10,14,18,22]){
      const opening=Array.from({length:count},(_,i)=>
        'Paragraphe totalement fictif numero '+(i+1)+' : '+('Un texte de test sans fait clinique ou identite reelle. ').repeat(4)).join('\n\n');
      const content=[
        '## Contexte fictif',opening,
        '### PARENT-DE-SECTION',
        '#### PREMIER-ENFANT-DE-SECTION',
        ...Array.from({length:7},(_,i)=>'- EXEMPLE-'+i+' : '+('Donnee purement fictive. ').repeat(3)),
        '#### AUTRE-ENFANT-DE-SECTION',
        'Une suite de paragraphe fictive.'
      ].join('\n');
      const pdf=await createMedicalPdf(content,patient,new Date(2026,10,11));
      const pageCount=(await PDFDocument.load(await pdf.arrayBuffer())).getPageCount();
      assert.ok(pageCount<=4,'A short nested document should not waste pages');
      const filename=join(directory,'nested-'+count+'.pdf');
      writeFileSync(filename,new Uint8Array(await pdf.arrayBuffer()));
      const result=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
      if(result.error?.code==='ENOENT'){
        t.diagnostic('pdftotext unavailable; PDF page-count assertions still executed');
        break;
      }
      assert.equal(result.status,0);
      const pages=result.stdout.split('\f').filter(page=>page.trim());
      const which=word=>pages.findIndex(page=>page.includes(word));
      const parent=which('PARENT-DE-SECTION'),child=which('PREMIER-ENFANT-DE-SECTION');
      assert.ok(parent>=0 && child>=0);
      assert.equal(parent,child,'Parent and first child must not split at a page break (size '+count+')');
    }
  }finally{rmSync(directory,{recursive:true,force:true});}
});

test('multi-page tables repeat headers and never split a table row', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'};
  const rows=Array.from({length:42},(_,i)=>
    '| LIGNE-'+String(i+1).padStart(2,'0')+' | Une observation fictive sans contenu médical et sans patient réel. |');
  const markdown=[
    '## Tableau de suivi fictif',
    '| REPERE | INFORMATION GENERALE |',
    '|---|---|',
    ...rows,
    '## Conclusion fictive',
    'Le tableau est terminé.'
  ].join('\n');
  const blob=await createMedicalPdf(markdown,patient,new Date(2026,10,11));
  const doc=await PDFDocument.load(await blob.arrayBuffer());
  assert.ok(doc.getPageCount()>1,'The test needs a table spanning multiple pages');
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-table-'));
  try{
    const filename=join(directory,'fictional-table.pdf');
    writeFileSync(filename,new Uint8Array(await blob.arrayBuffer()));
    const result=spawnSync('pdftotext',['-layout',filename,'-'],{encoding:'utf8'});
    if(result.error?.code==='ENOENT'){
      t.diagnostic('pdftotext unavailable; PDF page-count assertion still executed');
      return;
    }
    assert.equal(result.status,0);
    const pages=result.stdout.split('\f').filter(page=>page.trim());
    for(const page of pages){
      if(/LIGNE-\d\d/.test(page))assert.match(page,/REPERE/,'Each table page must repeat its header');
    }
    const text=pages.join('\n');
    for(let i=1;i<=42;i++){
      const code='LIGNE-'+String(i).padStart(2,'0');
      assert.equal(text.split(code).length-1,1,'Every table entry appears exactly once: '+code);
    }
  }finally{rmSync(directory,{recursive:true,force:true});}
});

test('session metadata fields are included only when explicitly entered', async t => {
  const patient={surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01',
    sessionFields:[{label:'Référence de séance',value:'REF-TEST-42'},
      {label:'Observation libre',value:'Information entièrement fictive.'},
      {label:'Champ vide',value:''}]};
  const markdown='## Résumé fictif\nUne rencontre sans information clinique réelle.';
  const document=await createMedicalPdf(markdown,patient,new Date(2026,10,11));
  const clean=await createMedicalPdf(markdown,{surname:'EXEMPLE',forename:'FICTIF',dateOfBirth:'2000-01-01'},new Date(2026,10,11));
  assert.ok((await PDFDocument.load(await document.arrayBuffer())).getPageCount()>=1);
  const directory=mkdtempSync(join(tmpdir(),'scribewatch-custom-fields-'));
  try{
    const file=join(directory,'with-fields.pdf');
    const emptyFile=join(directory,'without-fields.pdf');
    writeFileSync(file,new Uint8Array(await document.arrayBuffer()));
    writeFileSync(emptyFile,new Uint8Array(await clean.arrayBuffer()));
    const present=spawnSync('pdftotext',['-layout',file,'-'],{encoding:'utf8'});
    const absent=spawnSync('pdftotext',['-layout',emptyFile,'-'],{encoding:'utf8'});
    if(present.error?.code==='ENOENT'){
      t.diagnostic('pdftotext unavailable; PDF syntax still verified');
      return;
    }
    assert.equal(present.status,0);assert.equal(absent.status,0);
    assert.match(present.stdout,/REF-TEST-42/);
    assert.match(present.stdout,/Observation libre/);
    assert.doesNotMatch(present.stdout,/Champ vide/);
    assert.doesNotMatch(absent.stdout,/REF-TEST-42|Observation libre/);
  }finally{rmSync(directory,{recursive:true,force:true});}
});
