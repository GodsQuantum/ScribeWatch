/** A reduced navigation surface; advanced features are never disabled. */
export type UiMode='simple'|'advanced';
export type View='home'|'quick'|'live'|'workflows'|'jobs'|'settings';
const simpleItems=[
  {id:'home',label:'Home',glyph:'⌂'},
  {id:'live',label:'REC',glyph:'●'},
  {id:'jobs',label:'Documents',glyph:'≡'},
  {id:'settings',label:'Settings',glyph:'⚙'}
] as const;
const advancedItems=[
  {id:'home',label:'Home',glyph:'⌂'},
  {id:'quick',label:'Quick',glyph:'✦'},
  {id:'live',label:'REC',glyph:'●'},
  {id:'workflows',label:'Workflows',glyph:'↻'},
  {id:'jobs',label:'Documents',glyph:'≡'},
  {id:'settings',label:'Settings',glyph:'⚙'}
] as const;
export const normalizeMode=(value:unknown):UiMode=>value==='advanced'?'advanced':'simple';
export function navigationItems(mode:UiMode){return mode==='advanced'?advancedItems:simpleItems;}
export const navigationAllows=(mode:UiMode,view:string)=>navigationItems(mode).some(item=>item.id===view);
export function normalizeView(value:unknown,mode:UiMode):View {
  if(value==='ai'||value==='providers')return 'settings';
  if(value==='dashboard')return 'home';
  return typeof value==='string'&&navigationAllows(mode,value)?value as View:'live';
}
