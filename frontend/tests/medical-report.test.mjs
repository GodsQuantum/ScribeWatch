import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import assert from 'node:assert/strict';
const s=readFileSync(new URL('../src/lib/medical-report.ts',import.meta.url),'utf8');
test('patient data stays client side',()=>{assert.ok(!s.includes('localStorage'));assert.ok(!s.includes('fetch('));});
test('filenames based on surname then given name',()=>{assert.ok(s.includes("a+b+'_'"));assert.ok(s.includes('.slice(0,3)'));});

test('practice identity can be cached without patient identity or clinical content',()=>{
  const live=readFileSync(new URL('../src/lib/views/LiveRecordView.svelte',import.meta.url),'utf8');
  assert.ok(live.includes('scribewatch.practice.directory.v2'));
  assert.ok(live.includes("name=practitionerName.trim().slice(0,160)"));
  assert.ok(live.includes("address:practiceAddress.trim().slice(0,240)"));
  assert.ok(live.includes("contact:practiceContact.trim().slice(0,200)"));
  assert.ok(live.includes('localStorage.setItem(practiceDirectoryKey,JSON.stringify(updated))'));
  const profile=live.split('function savePracticeIdentity(){')[1]?.split('function clearPracticeIdentity(){')[0];
  assert.ok(profile);
  for(const personal of ['patientSurname','patientForename','patientBirthDate','consultationDate']){
    assert.ok(!profile.includes(personal),'Patient data must not enter saved practice profile');
  }
});

test('clinical encounter data is frozen per completed job before export',()=>{
  const live=readFileSync(new URL('../src/lib/views/LiveRecordView.svelte',import.meta.url),'utf8');
  assert.ok(live.includes('const encounter=snapshotEncounter()'));
  assert.ok(live.includes('metadataByJob.set(submitted.id,encounter)'));
  assert.ok(live.includes('metadataByJob.get(current.id)??snapshotEncounter()'));
  assert.ok(live.includes('sessionFields:encounter.sessionFields'));
  assert.ok(live.includes('medicalReportEnabled:'));// part of the isolated snapshot
});
