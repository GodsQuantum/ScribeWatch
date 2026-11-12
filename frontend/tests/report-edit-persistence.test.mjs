import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {PDFDocument} from 'pdf-lib';
import {splitEditable,composeEditable} from '../src/lib/report-edit.ts';
import {structuredMedicalBody,createMedicalPdf} from '../src/lib/medical-report.ts';
import {buildSavedClinicalPdf} from '../src/lib/clinical-report-export.ts';

const ORIGINAL = [
  '---','source: "consultation.m4a"','---','',
  '# Compte rendu','',
  '## Structured notes','',
  '## Motif','',
  'Brouillon initial.','',
  '## Soins','',
  'Geste à confirmer.','',
  '## Transcript','',
  'Bonjour, voici les paroles originales du patient.', ''
].join('\n');

test('report editor preserves immutable source and frontmatter, PDF sees edited version',async()=>{
  const parts=splitEditable(ORIGINAL);
  assert.equal(parts.section,true);
  const updated=composeEditable(ORIGINAL,'## Motif\n\nMotif corrigé.\n\n## Soins\n\nGeste non réalisé.');
  assert.equal(updated.split('## Transcript\n')[1],ORIGINAL.split('## Transcript\n')[1]);
  assert.ok(updated.includes('source: "consultation.m4a"'));
  const medical=structuredMedicalBody(updated);
  assert.ok(medical.includes('Motif corrigé.'));
  assert.ok(medical.includes('Geste non réalisé.'));
  assert.ok(!medical.includes('Brouillon initial.'));
  assert.ok(!medical.includes('voici les paroles originales'));
  const pdf=await createMedicalPdf(updated,{surname:'TEST',forename:'EXEMPLE',dateOfBirth:''});
  const loaded=await PDFDocument.load(await pdf.arrayBuffer());
  assert.ok(loaded.getPageCount()>=1);
});

test('plain imported reports remain editable and an untouched report never loses source',()=>{
  assert.equal(splitEditable('Note initiale').section,false);
  assert.equal(composeEditable('Note initiale','Correction'), 'Correction');
  assert.equal(splitEditable(ORIGINAL).body,splitEditable(composeEditable(ORIGINAL,splitEditable(ORIGINAL).body)).body);
});

test('saved editor is accessible from recording, quick transcription and document history',()=>{
  const root='src/lib/views/';
  for(const filename of ['JobsView.svelte','QuickTranscribeView.svelte','LiveRecordView.svelte']){
    const content=readFileSync(root+filename,'utf8');
    assert.match(content,/ReportEditor/);
  }
  const component=readFileSync('src/lib/components/ReportEditor.svelte','utf8');
  assert.match(component,/saveJobReport/);
  assert.match(component,/Voir la version initiale/);
  assert.match(component,/Exporter PDF modifié/);
});

test('re-opened report with corrected metadata generates native medical PDF with revision filename',async()=>{
  const markdown=composeEditable(ORIGINAL,'## Motif\n\nConsultation corrigée.');
  const job={
    id:'test-local-job',createdAtMs:Date.parse('2026-11-12T11:00:00Z'),
    structuredProfileId:'dental-consultation-complete-fr',
    structuredProfileName:'Compte rendu complet dentaire',
    structuredModel:'Qwen3.5-4B-Q4_K_M',
    quick:{live:{sessionId:'local-session',startedAtMs:Date.parse('2026-11-12T09:00:00Z')}}
  };
  const report={markdown,revision:2};
  const encounter={
    surname:'TEST',forename:'PATIENT',dateOfBirth:'1990-01-01',
    consultationDate:'2026-11-12',practitioner:'Dr Demo',
    practiceAddress:'12 rue Exemple',practiceContact:'',sessionFields:[],
    medicalReportEnabled:true
  };
  const {blob,filename}=await buildSavedClinicalPdf(job,report,encounter);
  assert.match(filename,/^TESPAT_2026-11-12_v2\.pdf$/);
  const pdf=await PDFDocument.load(await blob.arrayBuffer());
  assert.equal(pdf.getTitle(),'Compte rendu de consultation');
  assert.ok(pdf.getPageCount()>=1);
});
