import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Snapshot, Mutation, VpnStatus, Capability, UpdateInfo } from '../types';
export const desktop = isTauri();
export async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (desktop) return invoke<T>(command, args);
  if (location.protocol === 'file:') throw new Error('请通过 http://127.0.0.1:1420 打开网页版；直接打开 index.html 无法连接后端。');
  let response: Response;
  try {
    response = await fetch(`/api/${command}`, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Worktable-Client': 'web' }, body: JSON.stringify(args) });
  } catch { throw new Error('无法连接本机后端，请运行 npm run dev 后重试。'); }
  let result: unknown;
  try { result = await response.json(); } catch { throw new Error('本机后端未就绪，请确认 npm run dev 正在运行。'); }
  if (!response.ok) {
    const message = typeof result === 'object' && result !== null && 'error' in result && typeof result.error === 'string' ? result.error : '请求未完成，请重试。';
    throw new Error(message);
  }
  return result as T;
}
export function pickFile(accept: string): Promise<File | null> {
  return new Promise(resolve => {
    const input = document.createElement('input');
    input.type = 'file'; input.accept = accept; input.hidden = true;
    const finish = (file: File | null) => { input.remove(); resolve(file); };
    input.onchange = () => finish(input.files?.[0] ?? null);
    input.oncancel = () => finish(null);
    document.body.append(input); input.click();
  });
}
async function readFile(accept: string): Promise<{ name: string; content: string } | null> {
  const file = await pickFile(accept);
  if (!file) return null;
  if (file.size > 2 * 1024 * 1024) throw new Error('文件不能超过 2 MB。');
  return { name: file.name, content: await file.text() };
}
async function uploadVpn(file: File): Promise<boolean> {
  if (!file.name.toLowerCase().endsWith('.ovpn')) throw new Error('请选择 .ovpn 配置文件。');
  if (file.size > 2 * 1024 * 1024) throw new Error('文件不能超过 2 MB。');
  return call<boolean>('import_vpn_profile', { name: file.name, content: await file.text() });
}
async function importVpn() {
  if (desktop) return call<boolean>('import_vpn_profile');
  const upload = await readFile('.ovpn');
  return upload ? call<boolean>('import_vpn_profile', upload) : false;
}
interface PublicConfig { schemaVersion: number; labName: string; services: unknown[]; nodes: unknown[] }
async function importLab() {
  if (desktop) return call<boolean>('import_lab_config');
  const upload = await readFile('.json');
  if (!upload) return false;
  let parsed: Record<string, unknown>;
  try { parsed = JSON.parse(upload.content) as Record<string, unknown>; } catch { throw new Error('配置包不是有效 JSON。'); }
  const config = await call<PublicConfig>('validate_lab_config', parsed);
  if (!window.confirm(`导入「${config.labName}」的 ${config.services.length} 个服务和 ${config.nodes.length} 个节点？将替换现有服务与节点，保留 Todo、日程和 VPN 配置。`)) return false;
  return call<boolean>('import_lab_config', parsed);
}
async function exportLab() {
  if (desktop) return call<boolean>('export_lab_config');
  const config = await call<PublicConfig>('export_lab_config');
  const url = URL.createObjectURL(new Blob([JSON.stringify(config, null, 2)], { type: 'application/json' }));
  const link = document.createElement('a'); link.href = url; link.download = 'lab-config.json';
  document.body.append(link); link.click(); link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  return true;
}
async function notifyPermission() {
  if (desktop) return call<string>('request_notifications');
  if (!('Notification' in window)) throw new Error('当前浏览器不支持系统通知，网页内仍会显示日程提醒。');
  const permission = await Notification.requestPermission();
  return permission === 'granted' ? 'Granted' : permission === 'denied' ? 'Denied' : 'Prompt';
}
export const api = {
  snapshot: () => call<Snapshot>('get_snapshot'),
  mutate: (mutation: Mutation) => call<Snapshot>('mutate', { mutation }),
  vpnStatus: () => call<VpnStatus>('vpn_status'),
  vpnCapability: () => call<Capability>('vpn_capability'),
  connect: (profileId: string) => call<void>('vpn_connect', { profileId }),
  disconnect: () => call<void>('vpn_disconnect'),
  importVpn, uploadVpn,
  deleteVpn: (profileId: string) => call<void>('delete_vpn_profile', { profileId }),
  importLab, exportLab,
  serviceUrl: (serviceId: string) => call<string>('service_url', { serviceId }),
  openService: (serviceId: string, bounds: Bounds) => call<void>('show_service', { serviceId, bounds }),
  hideServices: () => desktop ? call<void>('hide_services') : Promise.resolve(),
  resizeService: (bounds: Bounds) => call<void>('resize_service', { bounds }),
  resetService: (serviceId: string) => call<void>('reset_service', { serviceId }),
  openNode: (nodeId: string) => call<void>('open_node', { nodeId }),
  probeNode: (nodeId: string) => call<boolean>('probe_node', { nodeId }),
  health: () => call<boolean>('cluster_health'),
  claimReminders: () => call<[string, string][]>('claim_reminders'),
  notifyPermission,
  checkUpdate: () => call<UpdateInfo>('check_update'),
  openDownload: (url: string) => desktop ? call<void>('open_download', { url }) : Promise.resolve(window.open(url, '_blank', 'noopener,noreferrer')),
};
export interface Bounds { x: number; y: number; width: number; height: number }
