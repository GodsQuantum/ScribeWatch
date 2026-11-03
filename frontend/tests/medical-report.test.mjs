import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import assert from 'node:assert/strict';
const s=readFileSync(new URL('../src/lib/medical-report.ts',import.meta.url),'utf8');
test('patient data stays client side',()=>{assert.ok(!s.includes('localStorage'));assert.ok(!s.includes('fetch('));});
test('filenames based on surname then given name',()=>{assert.ok(s.includes("a+b+'_'"));assert.ok(s.includes('.slice(0,3)'));});
