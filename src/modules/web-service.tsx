import { useEffect, useState } from 'react';
import { ArrowUpRight, RefreshCw, Globe } from 'lucide-react';
import type { Service } from '../types';
import { api } from '../lib/api';
import { errorMessage } from '../lib/logic';

export function WebService({ service }: { service: Service }) {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState('');
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let disposed = false;
    void api.serviceUrl(service.id).then(url => {
      if (!disposed) setUrl(url);
    }).catch(e => { if (!disposed) setError(errorMessage(e)); });
    return () => { disposed = true; };
  }, [service.id, attempt]);
  return <section className="card web-service-card">
    <Globe size={36} /><h2>{service.name}</h2><p>{service.url}</p>
    <p>服务在浏览器新标签页打开。保留该标签页即可保留表单、页面和登录状态。</p>
    <p className="muted">网页版遵循服务自身的 CSP 和登录策略；内嵌 WebView 将在后续桌面打包阶段恢复。</p>
    {error ? <><p role="alert">{error}</p><button className="button secondary" onClick={() => { setError(''); setUrl(null); setAttempt(n => n + 1); }}><RefreshCw size={15} />重新检测</button></> : url ? <a className="button primary" href={url} target="_blank" rel="noopener noreferrer">打开 {service.name}<ArrowUpRight size={16} /></a> : <p role="status">正在检查访问条件…</p>}
  </section>;
}
