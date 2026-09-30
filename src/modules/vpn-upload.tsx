import { useRef, useState } from 'react';
import { UploadCloud, ShieldCheck, FileCheck2, CircleAlert, LoaderCircle } from 'lucide-react';
import { api } from '../lib/api';

export function VpnUpload({ refresh }: { refresh: () => Promise<void> }) {
  const input = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<{ name: string; message: string; success: boolean }[]>([]);
  async function upload(files: File[]) {
    if (busy || !files.length) return;
    setBusy(true);
    setResults([]);
    let imported = false;
    try {
      for (const file of files) {
        let message: string;
        let success = false;
        try {
          await api.uploadVpn(file);
          imported = true;
          message = '上传成功';
          success = true;
        } catch (error) { message = error instanceof Error ? error.message : '上传失败，请重试。'; }
        setResults(old => [...old, { name: file.name, message, success }]);
      }
      if (imported) {
        try { await refresh(); }
        catch { setResults(old => [...old, { name: '配置列表', success: false, message: '文件已上传，列表刷新失败，请刷新页面。' }]); }
      }
    } finally { setBusy(false); }
  }
  return <div className="vpn-upload" aria-busy={busy}>
    <div className={`vpn-dropzone${dragging ? ' is-dragging' : ''}${busy ? ' is-busy' : ''}`}
      onDragOver={e => { e.preventDefault(); if (!busy) setDragging(true); }}
      onDragLeave={e => { if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDragging(false); }}
      onDrop={e => { e.preventDefault(); setDragging(false); void upload(Array.from(e.dataTransfer.files)); }}>
      <input ref={input} className="vpn-file-input" aria-label="上传 VPN 配置文件" type="file" accept=".ovpn" multiple disabled={busy} tabIndex={-1} onChange={e => { const files = Array.from(e.target.files ?? []); e.target.value = ''; void upload(files); }} />
      <div className="vpn-upload-icon">{busy ? <LoaderCircle className="upload-spinner" size={28} /> : <UploadCloud size={28} strokeWidth={1.6} />}</div>
      <h3>{busy ? '正在上传配置' : dragging ? '松开鼠标，即可上传' : '将 .ovpn 文件拖到这里'}</h3>
      <p>支持多个配置文件 · 每个不超过 2 MB</p>
      <button type="button" className="button primary" disabled={busy} onClick={() => input.current?.click()}>{busy ? '上传中…' : '选择文件'}</button>
    </div>
    <div className="vpn-upload-note"><ShieldCheck size={17} /><div><strong>仅保存在本机，上传后不会自动连接</strong><p>请使用包含证书和私钥的完整 .ovpn 配置。</p></div></div>
    <ul className="vpn-upload-results" aria-live="polite">{results.map((result, index) => <li className={result.success ? 'upload-success' : 'upload-error'} key={index}>{result.success ? <FileCheck2 size={19} /> : <CircleAlert size={19} />}<div><strong>{result.name}</strong><p>{result.message}</p></div></li>)}</ul>
  </div>;
}
