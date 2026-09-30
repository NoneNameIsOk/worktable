import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import net from 'node:net';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const mode = process.argv[2] ?? 'dev';
const env = { ...process.env };
const localCargo = path.join(root, '.tools/cargo/bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo');
if (!env.CARGO_HOME && existsSync(localCargo)) {
  env.CARGO_HOME = path.join(root, '.tools/cargo');
  env.RUSTUP_HOME = path.join(root, '.tools/rustup');
  env.PATH = `${path.dirname(localCargo)}${path.delimiter}${env.PATH ?? ''}`;
}
const cargo = env.CARGO_HOME ? path.join(env.CARGO_HOME, 'bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo') : 'cargo';
const children = [];
let stopping = false;
function stop(code = 0) {
  if (stopping) return;
  stopping = true;
  for (const child of children) if (child.exitCode === null) child.kill('SIGTERM');
  process.exitCode = code;
}
function launch(command, args) {
  const child = spawn(command, args, { cwd: root, env, stdio: 'inherit', shell: false });
  children.push(child);
  child.on('error', e => { console.error(`启动失败：${e.message}`); stop(1); });
  return child;
}
function completed(child) {
  return new Promise(resolve => { child.once('exit', code => resolve(code ?? 1)); child.once('error', () => resolve(1)); });
}
async function available(port) {
  return new Promise(resolve => {
    const server = net.createServer();
    server.once('error', () => resolve(false));
    server.listen(port, '127.0.0.1', () => server.close(() => resolve(true)));
  });
}
process.on('SIGINT', () => stop());
process.on('SIGTERM', () => stop());
if (!(await available(1421)) || (mode === 'dev' && !(await available(1420)))) {
  console.error('Worktable 端口已占用（1420 / 1421）。请先停止此前的实例，不会连接到未知后端。');
  process.exitCode = 1;
} else {
  const build = launch(cargo, ['build', '--locked', '-p', 'worktable-server']);
  if (await completed(build) !== 0 || stopping) { stop(1); }
  else {
    const executable = path.join(root, 'target/debug', process.platform === 'win32' ? 'worktable-server.exe' : 'worktable-server');
    const backend = launch(executable, []);
    backend.once('exit', code => stop(code ?? 0));
    if (mode === 'dev') {
      const vite = launch(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js')]);
      vite.once('exit', code => stop(code ?? 0));
      console.log('网页版：http://127.0.0.1:1420（前端 + 本机 SQLite 后端）');
    } else {
      console.log('网页版：http://127.0.0.1:1421（运行 npm run build 后使用）');
    }
  }
}
