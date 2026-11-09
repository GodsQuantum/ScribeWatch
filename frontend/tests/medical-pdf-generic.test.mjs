import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMedicalPdf, medicalFilename, structuredMedicalBody, uniqueMedicalFilename } from '../src/lib/medical-report.ts';
import { PDFDocument } from 'pdf-lib';

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
  assert.match(pdf.getSubject(), /session-2026-11-03-A/);
  const page=pdf.getPages()[0];
  assert.ok(page.getSize().width>590);

  assert.ok(blob.size > 1000);
  assert.ok(blob.type === 'application/pdf');
  assert.ok(pdf.getPageCount() < 8, 'Simple consultation should not spill into excessive pages');
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
