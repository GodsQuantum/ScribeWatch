<script lang="ts">
  import {api} from '$lib/api';
  import {saveMarkdownLocally} from '$lib/local-save';
  import {splitEditable,composeEditable} from '$lib/report-edit';
  import type {ExportFormat,ReportEncounter,SavedReport} from '$lib/types';

  export let clinical=false;
  export let jobId:string;
  export let notify:(type:'success'|'error',message:string)=>void=()=>{};
  export let onexport:((format:ExportFormat)=>Promise<void>)|undefined=undefined;

  let open=false;
  let loading=false;
  let saving=false;
  let exporting='';
  let report:SavedReport|null=null;
  let text='';
  let original=false;
  let showHistory=false;
  const emptyEncounter=():ReportEncounter=>({
    surname:'',forename:'',dateOfBirth:'',consultationDate:'',
    practitioner:'',practiceAddress:'',practiceContact:'',
    sessionFields:[],medicalReportEnabled:true
  });
  let savedEncounter:ReportEncounter|null=null;
  let encounter:ReportEncounter=emptyEncounter();
  let patientSaving=false;
  $: patientDirty=clinical&&JSON.stringify(encounter)!==
    JSON.stringify(savedEncounter??emptyEncounter());


  $: structure=splitEditable(report?.markdown??'');
  $: originalStructure=splitEditable(report?.originalMarkdown??'');
  $: dirty=!!report&&text!==structure.body;
  $: editableFull=report?composeEditable(report.markdown,text):'';

  async function begin(){
    if(open){if((dirty||patientDirty)&&!confirm('Modifications non enregistrées : fermer ?'))return;open=false;return;}
    open=true;loading=true;original=false;
    try{
      const [loaded,patient]=await Promise.all([
        api.jobReport(jobId),clinical?api.jobEncounter(jobId):Promise.resolve(null)
      ]);
      report=loaded;
      text=splitEditable(loaded.markdown).body;
      savedEncounter=patient;
      encounter=patient?structuredClone(patient):emptyEncounter();
    }catch(error){open=false;notify('error',error instanceof Error?error.message:String(error));}
    finally{loading=false;}
  }
  async function save(){
    if(!report||saving||!dirty)return;
    if(!text.trim()){notify('error','Le compte-rendu ne peut pas être vide.');return;}
    saving=true;
    try{
      report=await api.saveJobReport(jobId,editableFull,report.revision);
      text=splitEditable(report.markdown).body;
      notify('success','Compte-rendu corrigé enregistré dans ScribeWatch (version '+report.revision+').');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
    finally{saving=false;}
  }
  async function savePatient(){
    if(!clinical||patientSaving||!patientDirty)return;
    patientSaving=true;
    try{
      const saved=await api.saveJobEncounter(jobId,encounter,savedEncounter);
      savedEncounter=structuredClone(saved);
      encounter=structuredClone(saved);
      notify('success','Identité et coordonnées de consultation conservées localement.');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
    finally{patientSaving=false;}
  }
  function restore(markdown:string){
    if(!confirm('Charger cette ancienne version dans l’éditeur ? Enregistrer ensuite pour confirmer.'))return;
    text=splitEditable(markdown).body;
    original=false;
  }
  async function exportSaved(format:ExportFormat){
    if(!report||dirty||patientDirty||saving||patientSaving||exporting)return;
    exporting=format;
    try{
      if(onexport){await onexport(format);}
      else if(format==='pdf'){
        const {buildSavedNotePdf}=await import('$lib/note-pdf');
        const file=await buildSavedNotePdf(report);
        await saveMarkdownLocally(file.filename,file.blob);
      }else{
        const file=await api.jobExport(jobId,format);
        await saveMarkdownLocally(file.filename,file.blob);
      }
      if(!onexport)notify('success','Version enregistrée exportée ('+format.toUpperCase()+').');
    }catch(error){notify('error',error instanceof Error?error.message:String(error));}
    finally{exporting='';}
  }
</script>

<div class="report-editor" data-report-editor>
  <button type="button" class="btn compact" aria-expanded={open} on:click={begin}>
    {open?'Fermer l’éditeur':'Modifier le compte-rendu'}
  </button>
  {#if open}
    <section class="card report-edit-panel" aria-label="Éditeur de compte-rendu enregistré">
      {#if loading}
        <p>Chargement du compte-rendu local…</p>
      {:else if report}
        <div class="row wrap">
          <strong>Compte-rendu enregistré · version {report.revision}</strong>
          {#if report.revision>0}<span class="help">Dernière modification : {new Date(report.updatedAtMs).toLocaleString()}</span>{/if}
        </div>
        <p class="help">La génération initiale et la transcription Parakeet sont conservées localement. Les corrections doivent être validées par le praticien.</p>
        {#if original}
          <pre class="report-original">{originalStructure.body}</pre>
        {:else}
          <label class="field-label" for={'edit-report-'+jobId}>Compte-rendu modifiable (Markdown)</label>
          <textarea id={'edit-report-'+jobId} class="input report-textarea" rows="22" spellcheck="true" bind:value={text}></textarea>
          {#if dirty}<p class="help">Modifications non enregistrées. Enregistrer avant export.</p>{/if}
        {/if}
        {#if clinical}
          <details class="report-patient-panel">
            <summary>Informations du patient et du cabinet pour le PDF</summary>
            <p class="help">Ces informations sont conservées uniquement dans le dossier local de cette consultation, pas dans les profils ni dans le cloud.</p>
            <div class="report-patient-grid">
              <label>Nom du patient<input class="input" maxlength="512" bind:value={encounter.surname}/></label>
              <label>Prénom<input class="input" maxlength="512" bind:value={encounter.forename}/></label>
              <label>Naissance<input class="input" type="date" bind:value={encounter.dateOfBirth}/></label>
              <label>Consultation<input class="input" type="date" bind:value={encounter.consultationDate}/></label>
              <label>Praticien<input class="input" maxlength="512" bind:value={encounter.practitioner}/></label>
              <label>Contact<input class="input" maxlength="512" bind:value={encounter.practiceContact}/></label>
              <label class="report-patient-wide">Adresse du cabinet<input class="input" maxlength="512" bind:value={encounter.practiceAddress}/></label>
            </div>
            <button type="button" class="btn primary" disabled={!patientDirty||patientSaving} on:click={savePatient}>
              {patientSaving?'Enregistrement…':'Enregistrer les informations patient'}
            </button>
            {#if patientDirty}<p class="help">Modifications des informations patient non enregistrées.</p>{/if}
          </details>
        {/if}
        <div class="row wrap">
          <button type="button" class="btn primary" disabled={saving||!dirty||original} on:click={save}>
            {saving?'Enregistrement…':'Enregistrer les modifications'}
          </button>
          <button type="button" class="btn" on:click={()=>original=!original}>
            {original?'Revenir à l’édition':'Voir la version initiale'}
          </button>
          <button type="button" class="btn" disabled={!report.history.length} on:click={()=>showHistory=!showHistory}>
            Historique ({report.history.length})
          </button>
          <button type="button" class="btn" disabled={!!exporting||dirty||patientDirty||saving||patientSaving} on:click={()=>exportSaved('pdf')}>
            {exporting==='pdf'?'Export…':'Exporter PDF modifié'}
          </button>
          <button type="button" class="btn" disabled={!!exporting||dirty||patientDirty||saving||patientSaving} on:click={()=>exportSaved('md')}>
            Exporter Markdown modifié
          </button>
        </div>
        {#if showHistory}
          <div class="report-history">
            <div class="row wrap">
              <span>Génération initiale · version 0</span>
              <button type="button" class="btn compact" on:click={()=>report&&restore(report.originalMarkdown)}>Charger</button>
            </div>
            {#each [...report.history].reverse() as revision (revision.revision)}
              <div class="row wrap">
                <span>Version {revision.revision} · {new Date(revision.savedAtMs).toLocaleString()}</span>
                <button type="button" class="btn compact" on:click={()=>restore(revision.markdown)}>Charger</button>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </section>
  {/if}
</div>

<style>
  .report-editor{flex:1 1 100%;width:100%;min-width:0;}
  .report-edit-panel{margin-top:12px;padding:18px;display:flex;flex-direction:column;gap:12px;}
  .report-textarea{font:14px/1.55 ui-monospace,Consolas,monospace;min-height:360px;width:100%;resize:vertical;white-space:pre-wrap;}
  .report-original{white-space:pre-wrap;overflow:auto;max-height:380px;font:13px/1.5 ui-monospace,Consolas,monospace;padding:12px;border:1px solid var(--border, #777);border-radius:6px;}
  .report-history{display:flex;flex-direction:column;gap:8px;padding:8px;border-top:1px solid var(--border, #777);}
  .report-history>div{justify-content:space-between;}
  .report-patient-panel{border-top:1px solid var(--border,#777);padding:12px 0;}
  .report-patient-panel summary{cursor:pointer;font-weight:600;}
  .report-patient-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:12px;margin:12px 0;}
  .report-patient-grid label{display:flex;flex-direction:column;gap:5px;}
  .report-patient-wide{grid-column:1/-1;}
</style>
