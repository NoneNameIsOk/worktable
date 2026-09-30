import type { Service, SidebarItem, Todo, VpnState } from '../types';
export const moduleNames: Record<string, string> = { home: '首页', services: '服务', nodes: '节点', experiments: '实验', calendar: '日程', plugins: '插件', settings: '设置' };
export const vpnLabels: Record<VpnState, string> = { Disconnected: '未连接', Connecting: '连接中', Connected: '已连接', Reconnecting: '重连中', Disconnecting: '断开中', Failed: '连接失败' };
export function canConnect(s: VpnState) { return s === 'Disconnected' || s === 'Failed'; }
export function serviceAccess(s: Service, vpn: VpnState): 'disabled' | 'vpn' | 'open' { return !s.enabled ? 'disabled' : s.requiresVpn && vpn !== 'Connected' ? 'vpn' : 'open'; }
export function pendingTodos(todos: Todo[]) { return todos.filter(t => !t.completed); }
export function reorder(items: SidebarItem[], active: string, over: string): string[] {
  const ids = items.map(x => x.id); const a = ids.indexOf(active); const b = ids.indexOf(over);
  if (a < 0 || b < 0 || a === b) return ids;
  ids.splice(b, 0, ...ids.splice(a, 1)); return ids;
}
export function dateTime(s: string) { return new Intl.DateTimeFormat('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }).format(new Date(s)); }
export function localInput(iso: string) { const d = new Date(iso); return new Date(d.getTime() - d.getTimezoneOffset() * 60000).toISOString().slice(0, 16); }
export function errorMessage(e: unknown) { return typeof e === 'string' ? e : e instanceof Error ? e.message : '操作未完成，请重试。'; }
