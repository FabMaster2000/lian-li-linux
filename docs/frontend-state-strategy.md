# Frontend State Strategy

## Decision
The frontend does not use optimistic writes.

Every write flow waits for a successful backend response first and only then updates local UI state with backend-confirmed data.

## Rules
- Never speculate locally before the API confirms the write.
- If the write endpoint already returns the affected resource state, that response becomes the new local source of truth.
- If a write endpoint only returns an action result, the UI stores that result and keeps editable draft state separate from readonly backend state.
- Background refreshes may update readonly information, but they must not overwrite unfinished user input.

## Applied To
- Lighting:
  - `POST /api/devices/:id/lighting/effect` returns the updated lighting state.
  - `frontend/src/hooks/useLightingWorkbenchData.ts` adopts that response after success.
- Fans:
  - `POST /api/devices/:id/fans/manual` returns the updated fan state.
  - `frontend/src/hooks/useFansWorkbenchData.ts` adopts that response after success and preserves previous RPM telemetry if the backend omits it.
- Profiles:
  - `POST /api/profiles` updates the profile list only after the backend returns the created profile.
  - `DELETE /api/profiles/:id` removes a profile only after the backend confirms deletion.
  - `POST /api/profiles/:id/apply` stores the backend apply result only after success.

## Why
- The backend is allowed to normalize values.
- Hardware-facing writes may quantize or merge state.
- A server-confirmed model avoids UI drift and avoids showing states that were never actually applied.

## Acceptance Mapping For Phase 9
- Lighting color and effect writes are covered in `frontend/src/pages/LightingPage.test.tsx` and `frontend/src/hooks/useLightingWorkbenchData.test.tsx`.
- Fan writes are covered in `frontend/src/pages/FansPage.test.tsx` and `frontend/src/hooks/useFansWorkbenchData.test.tsx`.
- Profile create/delete/apply flows are covered in `frontend/src/pages/ProfilesPage.test.tsx` and `frontend/src/hooks/useProfilesWorkbenchData.test.tsx`.
