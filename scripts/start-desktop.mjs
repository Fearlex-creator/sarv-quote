import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const binary = fileURLToPath(new URL(`../desktop/src-tauri/target/release/sarv-desktop${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url));
if (!existsSync(binary)) {
  console.error('Build the desktop app first: pnpm desktop:build --no-bundle');
  process.exit(1);
}
const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
