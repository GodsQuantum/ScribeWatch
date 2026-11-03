import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

const root=new URL('../',import.meta.url);
const read=(name)=>readFileSync(new URL(name,root),'utf8');

test('theme palette control is shared and stored per device, without storing recordings',()=>{
  const shell=read('src/lib/components/AppShell.svelte');
  const css=read('src/app.css');
  for(const id of ['night','arctic','slate','sand','violet']){
    assert.ok(shell.includes("id:'"+id+"'"),'Missing theme '+id);
    if(id!=='night')assert.ok(css.includes('data-sw-theme="'+id+'"'));
  }
  assert.match(shell,/document\.documentElement\.dataset\.swTheme/);
  assert.match(shell,/localStorage\.setItem\('scribewatch:theme'/);
  assert.doesNotMatch(shell,/localStorage\.setItem\([^)]*(?:audio|patient|transcript)/i);
});

test('workspace UI offers generic named groups, inbox and navigation',()=>{
  const shell=read('src/lib/components/AppShell.svelte');
  for(const label of ['All recordings','Inbox','Space name','General','Meeting','Project']){
    assert.ok(shell.includes(label),'Missing '+label);
  }
  assert.match(shell,/oncreateSpace/);
  assert.match(shell,/ondeleteSpace/);
  assert.match(shell,/membershipIndex/);
  assert.match(shell,/recent\.filter/);
  assert.match(shell,/window\.addEventListener\('keydown'/);
});

test('new quick and live jobs assign the active workspace without affecting the job',()=>{
  for(const file of ['QuickTranscribeView.svelte','LiveRecordView.svelte']){
    const source=read('src/lib/views/'+file);
    assert.match(source,/selectedSpace/);
    assert.match(source,/onassign\(submitted\.id,selectedSpace\)/);
  }
});

test('job history is space-filtered, search-normalized and progressively rendered',()=>{
  const source=read('src/lib/views/JobsView.svelte');
  assert.match(source,/new Map\(assignments\.map/);
  assert.match(source,/selectedSpace==='unassigned'/);
  assert.match(source,/normalized\(job\.originalName\)/);
  assert.match(source,/paged=visible\.slice\(0,shown\)/);
  assert.match(source,/shown=Math\.min\(visible\.length,shown\+30\)/);
  assert.match(source,/onassign\(jobId,value\|\|null\)/);
});

test('all workspace state is stored by backend API, not in browser localStorage',()=>{
  const api=read('src/lib/api.ts');
  for(const route of ['/api/v1/spaces','/api/v1/spaces/assignments','/space']){
    assert.ok(api.includes(route),route);
  }
  const page=read('src/routes/+page.svelte');
  assert.match(page,/api\.spaceAssignments\(\)/);
  assert.match(page,/api\.createSpace/);
  assert.match(page,/api\.assignJobSpace/);
  assert.match(page,/api\.deleteSpace/);
  assert.doesNotMatch(page,/localStorage\.setItem\('scribewatch:.*(?:job|transcript|patient)/i);
});
