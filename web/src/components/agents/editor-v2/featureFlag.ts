/**
 * Feature flag for the M3 AgentSpec-v2 editor modules.
 *
 * Enable via any of:
 * - URL `?editor=v2` (or `&editor=v2`)
 * - localStorage `neomind.agentEditorV2` = `1`
 * - Vite env `VITE_AGENT_EDITOR_V2=true`
 */
export const AGENT_EDITOR_V2_STORAGE_KEY = 'neomind.agentEditorV2'

export function isAgentEditorV2Enabled(
  search: string = typeof window !== 'undefined' ? window.location.search : '',
): boolean {
  if (import.meta.env.VITE_AGENT_EDITOR_V2 === 'true') return true
  try {
    if (typeof localStorage !== 'undefined' && localStorage.getItem(AGENT_EDITOR_V2_STORAGE_KEY) === '1') {
      return true
    }
  } catch {
    // private mode / blocked storage
  }
  const params = new URLSearchParams(search.startsWith('?') ? search : `?${search}`)
  return params.get('editor') === 'v2'
}
