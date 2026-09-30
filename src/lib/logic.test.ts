import { describe, expect, it } from 'vitest';
import { canConnect, pendingTodos, reorder, serviceAccess, vpnLabels } from './logic';
import type { Service, VpnState } from '../types';
const service: Service = { id: 's1', name: 'ClearML', url: 'https://example.org', icon: 'C', requiresVpn: true, favorite: true, pinnedToSidebar: false, sortOrder: 0, enabled: true };
describe('service access and VPN UI state', () => {
  it('requires an actual Connected state', () => { for (const state of ['Disconnected', 'Connecting', 'Reconnecting', 'Disconnecting', 'Failed'] as VpnState[]) expect(serviceAccess(service, state)).toBe('vpn'); expect(serviceAccess(service, 'Connected')).toBe('open'); });
  it('respects disabled services independently of VPN', () => { expect(serviceAccess({ ...service, enabled: false }, 'Connected')).toBe('disabled'); expect(serviceAccess({ ...service, requiresVpn: false }, 'Disconnected')).toBe('open'); });
  it('only enables connect from terminal states and translates all states', () => { expect(canConnect('Failed')).toBe(true); expect(canConnect('Connecting')).toBe(false); for (const s of Object.keys(vpnLabels) as VpnState[]) expect(vpnLabels[s]).not.toBe(''); });
});
describe('local productivity', () => {
  it('filters completed todos without mutation', () => { const todos = [{ id: '1', title: '实验', completed: false, dueDate: null }, { id: '2', title: '组会', completed: true, dueDate: null }]; expect(pendingTodos(todos).map(t => t.id)).toEqual(['1']); expect(todos).toHaveLength(2); });
  it('reorders all navigation identities without losing builtins', () => { const items = ['home', 'service:a', 'settings'].map((id, sortOrder) => ({ id, sortOrder })); expect(reorder(items, 'settings', 'home')).toEqual(['settings', 'home', 'service:a']); expect(items[0].id).toBe('home'); expect(reorder(items, 'unknown', 'home')).toEqual(items.map(i => i.id)); });
});
