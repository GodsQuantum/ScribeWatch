import test from 'node:test';
import assert from 'node:assert/strict';
import {sanitizeSessionSchemas,sanitizePracticePresets,sessionFieldPairs} from '../src/lib/session-fields.ts';

test('custom field definitions are generic and session answers are never saved in schemas',()=>{
  const source={
    meeting:[{id:'a',label:'Reference de reunion',type:'text'},{id:'b',label:'Date de revue',type:'date'}],
    consultation:[{id:'c',label:'Information libre',type:'multiline'}]
  };
  const schemas=sanitizeSessionSchemas(source);
  assert.deepEqual(Object.keys(schemas),['meeting','consultation']);
  assert.equal(schemas.meeting.length,2);
  const values={a:'  ID-TEST-123  ',b:'',c:'Information fictive'};
  assert.deepEqual(sessionFieldPairs(schemas.meeting,values),[{label:'Reference de reunion',value:'ID-TEST-123'}]);
  assert.ok(!JSON.stringify(schemas).includes('ID-TEST-123'),'Session values are never part of persisted form definitions');
});
test('sanitizers discard malicious types and keep several independent professionals',()=>{
  assert.deepEqual(sanitizeSessionSchemas({other:[
    {id:'x',label:'test',type:'text'},
    {id:'x',label:'duplicate',type:'date'},
    {id:'y',label:'unknown',type:'script'}
  ]}).other,[{id:'x',label:'test',type:'text'}]);
  assert.deepEqual(sanitizePracticePresets([
    {id:'one',name:'Dr Exemple A',address:'Cabinet A',contact:'contact A'},
    {id:'two',name:'Dr Exemple B',address:'Cabinet B',contact:'contact B'},
    {id:'one',name:'duplicate'}
  ]).map(item=>item.name),['Dr Exemple A','Dr Exemple B']);
});
