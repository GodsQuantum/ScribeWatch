/** Export saved and possibly edited clinical reports with the same PDF
 * renderer as live recording, without sending patient data to a service. */
import type {Job,ReportEncounter,SavedReport} from './types';

export async function buildSavedClinicalPdf(
  job:Job,report:SavedReport,encounter:ReportEncounter|null
):Promise<{blob:Blob;filename:string}>{
  const {createMedicalPdf,medicalFilename}=await import('./medical-report');
  const profile=job.structuredProfileName;
  if(job.structuringError||!profile)throw new Error('Compte-rendu clinique structuré indisponible.');
  const existing=encounter??{
    surname:'',forename:'',dateOfBirth:'',consultationDate:'',practitioner:'',
    practiceAddress:'',practiceContact:'',sessionFields:[],medicalReportEnabled:true
  };
  const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(report.markdown));
  const sourceDocumentSha256=Array.from(new Uint8Array(digest),v=>v.toString(16).padStart(2,'0')).join('');
  const sessionMs=job.quick?.live?.startedAtMs??job.createdAtMs;
  const sessionDate=Number.isFinite(sessionMs)&&sessionMs>0?new Date(sessionMs):new Date();
  const override=/^\d{4}-\d{2}-\d{2}$/.test(existing.consultationDate)
    ?new Date(existing.consultationDate+'T12:00:00'):undefined;
  const date=override&&!Number.isNaN(override.getTime())?override:sessionDate;
  const metadata={
    surname:existing.surname,forename:existing.forename,dateOfBirth:existing.dateOfBirth,
    practitioner:existing.practitioner,practiceAddress:existing.practiceAddress,
    practiceContact:existing.practiceContact,sourceSessionRef:job.quick?.live?.sessionId??job.id,
    sourceDocumentSha256,structuredProfile:profile,structuredModel:job.structuredModel,
    sessionFields:existing.sessionFields
  };
  const blob=await createMedicalPdf(report.markdown,metadata,date);
  const base=medicalFilename(metadata,date);
  const suffix=report.revision>0?'_v'+report.revision:'';
  return {blob,filename:suffix?base.replace(/\.pdf$/i,suffix+'.pdf'):base};
}
