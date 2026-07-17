import { spawnSync } from 'node:child_process'

const expectedVersion = '0.13.1'
const versionResult = spawnSync('wasm-pack', ['--version'], {
  encoding: 'utf8',
  killSignal: 'SIGKILL',
  timeout: 10_000,
})

if (versionResult.error?.code === 'ENOENT') {
  console.error(
    `wasm-pack ${expectedVersion} is required. Install it once with: cargo install wasm-pack --version ${expectedVersion} --locked`,
  )
  process.exit(1)
}

if (versionResult.error) {
  if (versionResult.error.code === 'ETIMEDOUT') {
    console.error(`wasm-pack ${expectedVersion} failed to start within 10 seconds.`)
    process.exit(1)
  }
  console.error(`Unable to run wasm-pack: ${versionResult.error.message}`)
  process.exit(1)
}

if (versionResult.status !== 0) {
  if (versionResult.stderr) process.stderr.write(versionResult.stderr)
  process.exit(versionResult.status ?? 1)
}

const installedVersion = versionResult.stdout.trim().split(/\s+/).at(-1)
if (installedVersion !== expectedVersion) {
  console.error(
    `Expected wasm-pack ${expectedVersion}, but found ${installedVersion ?? 'an unknown version'}.`,
  )
  process.exit(1)
}

const buildResult = spawnSync(
  'wasm-pack',
  [
    'build',
    'packages/photon-engine',
    '--target',
    'web',
    '--out-dir',
    'pkg',
    '--no-default-features',
    '--features',
    'wasm',
    '--locked',
  ],
  {
    killSignal: 'SIGKILL',
    stdio: 'inherit',
    timeout: 14 * 60 * 1000,
  },
)

if (buildResult.error) {
  if (buildResult.error.code === 'ETIMEDOUT') {
    console.error('wasm-pack build exceeded 14 minutes and was stopped.')
    process.exit(1)
  }
  console.error(buildResult.error.message)
  process.exit(1)
}

process.exit(buildResult.status ?? 1)
