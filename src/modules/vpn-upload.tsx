import { useState } from 'react';
import { api } from '../lib/api';

export function VpnUpload({ refresh }: { refresh: () => Promise<void> }) {
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<{ name: string; message: string }[]>([]);
  async function upload(files: File[]) {
    if (busy || !files.length) return;
    setBusy(true);
    setResults([]);
    let imported = false;
    try {
      for (const file of files) {
        let message: string;
        try {
          await api.uploadVpn(file);
          imported = true;
          message = '上传成功';
        } catch (error) { message = error instanceof Error ? error.message : '上传失败，请重试。'; }
        setResults(old => [...old, { name: file.name, message }]);
      }
      if (imported) {
        try { await refresh(); }
        catch { setResults(old => [...old, { name: '配置列表', message: '文件已上传，列表刷新失败，请刷新页面。' }]); }
      }
    } finally { setBusy(false); }
  }
  return <div className="vpn-upload" onDragOver={e => e.preventDefault()} onDrop={e => { e.preventDefault(); void upload(Array.from(e.dataTransfer.files)); }} aria-busy={busy}>
    <label className="field"><span>选择或拖入 .ovpn 文件</span><input aria-label="上传 VPN 配置文件" type="file" accept=".ovpn" multiple disabled={busy} onChange={e => { const files = Array.from(e.target.files ?? []); e.target.value = ''; void upload(files); }} /></label>
    <p className="muted">支持一次上传多个文件，每个不超过 2 MB。上传后保存在本机，不会自动连接。证书和私钥需包含在配置文件内。</p>
    {busy && <p role="status">正在上传…</p>}
    <ul aria-live="polite">{results.map((result, index) => <li key={index}><strong>{result.name}</strong>：{result.message}</li>)}</ul>
  </div>;
}
