// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { call, api } from './api';
afterEach(() => vi.unstubAllGlobals());
describe('web transport', () => {
  it('uses same-origin POST with the required local-client header', async () => {
    const fetch = vi.fn().mockResolvedValue({ ok: true, json: async () => ({ todos: [] }) });
    vi.stubGlobal('fetch', fetch);
    expect(await call('get_snapshot')).toEqual({ todos: [] });
    expect(fetch).toHaveBeenCalledWith('/api/get_snapshot', expect.objectContaining({ method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Worktable-Client': 'web' }, body: '{}' }));
  });
  it('shows the backend validation error instead of claiming success', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, json: async () => ({ error: '配置包版本不支持。' }) }));
    await expect(call('import_lab_config')).rejects.toThrow('配置包版本不支持。');
  });
  it('gives a usable startup instruction on backend outage', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('Failed to fetch')));
    await expect(api.snapshot()).rejects.toThrow('npm run dev');
  });
  it('does not invoke native WebView commands for ordinary web navigation', async () => {
    const fetch = vi.fn(); vi.stubGlobal('fetch', fetch);
    await api.hideServices(); expect(fetch).not.toHaveBeenCalled();
  });
});
