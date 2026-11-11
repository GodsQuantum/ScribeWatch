import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMedicalPdf, medicalFilename, structuredMedicalBody, uniqueMedicalFilename, wrapRichText, patientFacingMedicalBody } from '../src/lib/medical-report.ts';
import { PDFDocument, StandardFonts } from 'pdf-lib';

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
