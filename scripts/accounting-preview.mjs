import {spawn} from 'node:child_process'
import {randomBytes} from 'node:crypto'
import {tmpdir} from 'node:os'
import {fileURLToPath} from 'node:url'
import {dirname,join} from 'node:path'

const root=dirname(dirname(fileURLToPath(import.meta.url)))
const token=randomBytes(24).toString('hex')
const env={...process.env,FREE_ACCOUNTING_PREVIEW_TOKEN:token,VITE_ACCOUNTING_PREVIEW_TOKEN:token,SARV_PREVIEW_DATA_DIR:process.env.SARV_PREVIEW_DATA_DIR||join(tmpdir(),'sarv-desktop-preview')}
const backend=spawn('cargo',['run','-p','sarv-quote','--bin','desktop-preview','--offline'],{cwd:root,env,stdio:'inherit'})
const frontend=spawn('pnpm',['--dir','web','run','dev','--host','127.0.0.1'],{cwd:root,env,stdio:'inherit'})
console.log(`Development-only SQLite preview: ${env.SARV_PREVIEW_DATA_DIR}`)
function stop(){backend.kill('SIGTERM');frontend.kill('SIGTERM')}
process.on('SIGINT',stop);process.on('SIGTERM',stop)
backend.on('exit',code=>{frontend.kill('SIGTERM');process.exitCode=code||0})
frontend.on('exit',code=>{backend.kill('SIGTERM');process.exitCode=code||0})
