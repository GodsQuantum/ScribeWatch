import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import assert from 'node:assert/strict';
const s=readFileSync(new URL('../src/lib/medical-report.ts',import.meta.url),'utf8');
test('patient data stays client side',()=>{assert.ok(!s.includes('localStorage'));assert.ok(!s.includes('fetch('));});
test('filenames based on surname then given name',()=>{assert.ok(s.includes("a+b+'_'"));assert.ok(s.includes('.slice(0,3)'));});

test('practice identity can be cached without patient identity or clinical content',()=>{
  const live=readFileSync(new URL('../src/lib/views/LiveRecordView.svelte',import.meta.url),'utf8');
  assert.ok(live.includes('scribewatch.practice.identity.v1'));
  assert.ok(live.includes('practitioner:practitionerName.trim()'));
  assert.ok(live.includes('address:practiceAddress.trim()'));
  assert.ok(live.includes('contact:practiceContact.trim()'));
  const profile=live.split('function savePracticeIdentity(){')[1]?.split('function clearPracticeIdentity(){')[0];
  assert.ok(profile);
  for(const personal of ['patientSurname','patientForename','patientBirthDate','consultationDate']){
    assert.ok(!profile.includes(personal),'Patient data must not enter saved practice profile');
  }
});
