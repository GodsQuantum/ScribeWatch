import type { ExportFormat, JobStatus } from '$lib/types';

/** A manually authorized folder may receive results, but an incomplete
 * clinical AI report must NEVER become a patient-facing PDF automatically. */
export function autoSaveFormats(
  status:JobStatus,
  destinationChosen:boolean,
  medicalReportEnabled:boolean,
  structuredReportCompleted:boolean
):ExportFormat[] {
  if(!destinationChosen||status!=='done')return [];
  return medicalReportEnabled&&structuredReportCompleted?['pdf','md']:['md'];
}
