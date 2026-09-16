#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
APP_PORT="${SCRIBEWATCH_E2E_PORT:-39100}"
MOCK_PORT="${SCRIBEWATCH_E2E_MOCK_PORT:-39101}"
APP_PID=""; MOCK_PID=""
cleanup(){
  [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
  [[ -n "$MOCK_PID" ]] && kill "$MOCK_PID" 2>/dev/null || true
  wait "$APP_PID" "$MOCK_PID" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT
mkdir -p "$TMP/config" "$TMP/data" "$TMP/media/watch" "$TMP/media/notes" "$TMP/media/archive" "$TMP/media/quick"

PORT="$MOCK_PORT" python3 "$ROOT/scripts/mock-transcription-server.py" >"$TMP/mock.log" 2>&1 & MOCK_PID=$!
(
  export SCRIBEWATCH_HOST=127.0.0.1 SCRIBEWATCH_PORT="$APP_PORT"
  export SCRIBEWATCH_CONFIG_DIR="$TMP/config" SCRIBEWATCH_DATA_DIR="$TMP/data"
  export SCRIBEWATCH_DIST_DIR="$ROOT/frontend/build" SCRIBEWATCH_ALLOWED_ROOTS="$TMP/media"
  export SCRIBEWATCH_SCAN_SECONDS=1 SCRIBEWATCH_FILE_STABILITY_MS=50 SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS=1
  exec "$ROOT/target/debug/scribewatch"
) >"$TMP/app.log" 2>&1 & APP_PID=$!
BASE="http://127.0.0.1:$APP_PORT/api/v1"
for _ in $(seq 1 80); do curl -fsS "$BASE/ready" >/dev/null 2>&1 && break; sleep .1; done
curl -fsS "$BASE/ready" >/dev/null || { cat "$TMP/app.log"; exit 1; }
provider_json=$(cat <<JSON
{"id":"provider","name":"Mock Provider","transcriptionUrl":"http://127.0.0.1:$MOCK_PORT/v1/audio/transcriptions","model":"test-model","timeoutSeconds":10,"enabled":true}
JSON
)
curl -fsS -X POST "$BASE/providers" -H 'content-type: application/json' -d "$provider_json" >/dev/null
workflow_json=$(python3 - "$TMP" <<'PY'
import json,sys
r=sys.argv[1]
print(json.dumps({"id":"workflow","name":"Voice notes","watchDir":r+"/media/watch","outputDir":r+"/media/notes","archiveDir":r+"/media/archive","tags":["ideas"],"providerId":"provider","model":"","language":"en","markdown":{"frontmatter":True,"transcriptHeading":"Transcript"},"enabled":True}))
PY
)
curl -fsS -X POST "$BASE/workflows" -H 'content-type: application/json' -d "$workflow_json" >/dev/null

wait_job(){
  local id="$1" status body
  for _ in $(seq 1 120); do
    body="$(curl -fsS "$BASE/jobs/$id")"
    status="$(printf '%s' "$body" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')"
    [[ "$status" == done ]] && return 0
    if [[ "$status" == error || "$status" == interrupted || "$status" == cancelled ]]; then printf '%s\n' "$body" >&2; return 1; fi
    sleep .1
  done
  echo "job $id timed out" >&2; return 1
}
latest_job(){ curl -fsS "$BASE/jobs" | python3 -c 'import json,sys; x=json.load(sys.stdin); print(x[0]["id"])'; }
# Workflow: watch -> titled Markdown in notes -> archive source.
printf 'workflow audio' >"$TMP/media/watch/workflow.m4a"
curl -fsS -X PUT "$BASE/workflows/workflow/scan" >/dev/null
workflow_id="$(latest_job)"; wait_job "$workflow_id"
[[ ! -e "$TMP/media/watch/workflow.m4a" ]]
[[ -f "$TMP/media/archive/workflow.m4a" ]]
workflow_note="$(find "$TMP/media/notes" -maxdepth 1 -name '*.md' -print -quit)"
[[ -n "$workflow_note" && -f "$workflow_note" ]]
grep -q '^title: "Remember to book the train tomorrow\."' "$workflow_note"
grep -q '^  - ideas$' "$workflow_note"
grep -q '^# Remember to book the train tomorrow\.$' "$workflow_note"

# Quick server file: publish another note, preserve source.
printf 'server quick audio' >"$TMP/media/quick/server.m4a"
server_payload=$(python3 - "$TMP" <<'PY'
import json,sys
r=sys.argv[1]
print(json.dumps({"sourcePath":r+"/media/quick/server.m4a","providerId":"provider","model":"","language":"en","outputKind":"server","outputDir":r+"/media/notes","frontmatter":True}))
PY
)
server_id="$(curl -fsS -X POST "$BASE/quick/server" -H 'content-type: application/json' -d "$server_payload" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')"
wait_job "$server_id"
[[ -f "$TMP/media/quick/server.m4a" ]]
[[ "$(find "$TMP/media/notes" -maxdepth 1 -name '*.md' | wc -l)" -ge 2 ]]
# Quick browser upload: return client Markdown, remove transient upload.
printf 'browser quick audio' >"$TMP/browser.m4a"
upload_response="$(curl -fsS -X POST "$BASE/quick/upload" \
  -F providerId=provider -F model= -F language=en -F outputKind=client -F frontmatter=true \
  -F "file=@$TMP/browser.m4a;filename=browser.m4a")"
upload_id="$(printf '%s' "$upload_response" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')"
wait_job "$upload_id"
headers="$TMP/headers.txt"; result="$TMP/client.md"
curl -fsS -D "$headers" "$BASE/jobs/$upload_id/markdown" -o "$result"
grep -qi 'content-disposition: attachment;' "$headers"
grep -q '^# Remember to book the train tomorrow\.$' "$result"
[[ -z "$(find "$TMP/data/quick-uploads" -type f -print -quit)" ]]
[[ "$(find "$TMP/data/quick-results" -type f | wc -l)" -eq 1 ]]

echo 'SCRIBEWATCH_V02_E2E=PASS'
