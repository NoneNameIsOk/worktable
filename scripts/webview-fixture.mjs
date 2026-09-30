// Local-only manual acceptance fixture, never loaded by the application itself.
import http from 'node:http';
const page = `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>WebView 验收页面</title><style>body{font:16px system-ui;padding:40px;color:#253047}input,button{padding:10px;margin:10px 0}small{display:block;margin-top:15px;color:#526587}</style><h1>WebView 验收页面</h1><p>用于验证页面保留、存储和远程 IPC 隔离。</p><label>页面内表单 <input id="form" placeholder="切换后应保留此文本"></label><div><button id="save">写入测试存储</button></div><small id="storage"></small><small id="ipc">正在检测 IPC</small><script>
const status = document.getElementById('storage');
const show = () => status.textContent = 'LocalStorage: ' + localStorage.getItem('worktable-test') + ' / Cookie: ' + document.cookie;
show();document.getElementById('save').onclick = () => {localStorage.setItem('worktable-test','persisted');document.cookie='worktable-test=persisted; Max-Age=3600; SameSite=Lax';show();};
(async()=>{const api=window.__TAURI_INTERNALS__;const el=document.getElementById('ipc');if(!api?.invoke){el.textContent='远程 IPC：不可用（隔离成功）';return;}try{await api.invoke('get_snapshot');el.textContent='失败：远程页面取得本地数据';}catch{el.textContent='远程 IPC：拒绝访问（隔离成功）';}})();
</script></html>`;
http.createServer((_, response) => { response.setHeader('Content-Type','text/html; charset=utf-8'); response.end(page); }).listen(17832,'127.0.0.1',()=>console.log('WebView fixture: http://127.0.0.1:17832'));
