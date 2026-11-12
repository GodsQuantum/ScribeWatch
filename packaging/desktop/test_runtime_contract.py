"""Prevent an older llama engine or an altered clinical recipe from being shipped silently.

Pure source/package contract. Real clinical quality still requires a PRIVATE GPU E2E.
"""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT=Path(__file__).resolve().parents[2]
LOCK=json.loads((ROOT/'packaging/desktop/llama-runtime-lock.json').read_text())
WF=(ROOT/'.github/workflows/standalone-windows.yml').read_text()
LLM=(ROOT/'src/llm.rs').read_text()
EMBEDDED=(ROOT/'src/api/embedded.rs').read_text()

class RuntimeContractTests(unittest.TestCase):
    def test_pinned_release_and_archive_hashes_are_exact(self):
        ll=LOCK['llama_cpp']
        self.assertEqual(ll['release'],'b12110')
        self.assertEqual(ll['commit'],'cd959bae99ff4bed22bef2604f45241c09bbb8c8')
        self.assertIn('gh release download '+ll['release'],WF)
        for asset,digest in ll['windows_amd64_assets_sha256'].items():
            self.assertRegex(digest,r'^[0-9a-f]{64}$')
            self.assertIn(asset,WF)
            self.assertIn(digest,WF)
        self.assertIn('cd959bae9',WF)
        self.assertIn('Assert packaged llama.cpp',WF)

    def test_builtin_profile_prompt_is_the_verbatim_tested_pilot(self):
        marker=re.search(r'pub\(crate\) const DENTAL_BEST_PILOT_PROMPT: &str = r#"(.*?)"#;',LLM,re.S)
        self.assertIsNotNone(marker)
        self.assertEqual(hashlib.sha256(marker.group(1).encode()).hexdigest(),
                         LOCK['reference_4b_quality_protocol']['pilot_prompt_sha256'])

    def test_local_inference_and_clinical_safety_policy(self):
        policy=LOCK['reference_4b_quality_protocol']
        for needle in ('stable_completion_seed','source_action_recall_pass',
                       'source_recall_policy','needs_long_form_source_coverage',
                       'clinical_sections','screen_clinical_report',
                       'enable_thinking','"temperature"','"top_p"'):
            self.assertIn(needle,LLM)
        self.assertIn('LOCAL_LLM_OUTPUT_TOKENS: usize = 5_000',LLM)
        self.assertIn('body["temperature"] = json!(0.1)',LLM)
        self.assertIn('body["top_p"] = json!(0.9)',LLM)
        self.assertIn('body["seed"] = json!(stable_completion_seed(model, system, user))',LLM)
        self.assertEqual(policy['context_tokens'],24576)
        self.assertIn('Some(mib) if mib >= 3_840 => 24_576',EMBEDDED)
        self.assertIn('Some("q4_0") => "q4_0"',EMBEDDED)
        self.assertIn('"--cache-type-k".into()',EMBEDDED)
        self.assertIn('"--cache-type-v".into()',EMBEDDED)

if __name__=='__main__':
    unittest.main()
