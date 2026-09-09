import { copyFileSync, existsSync, mkdirSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

// `tauri android init` renders the Gradle project from the CLI's own template,
// which carries Tauri's placeholder launcher icons. Nothing in the CLI copies
// src-tauri/icons/android into the generated project -- `tauri icon` writes
// into gen/android/app/src/main/res directly, and only when the project already
// exists -- so a build from a clean checkout, which is every CI build, ships
// the placeholder. Copy the committed icons over it after init.
const source = 'src-tauri/icons/android'
const target = 'src-tauri/gen/android/app/src/main/res'

if (!existsSync(target)) {
  throw new Error(`${target} does not exist; run \`tauri android init\` first`)
}

const isMipmap = (name) => name.startsWith('mipmap-')
const densities = readdirSync(source).filter(isMipmap)

// The manifest points at fixed resource names, so a density folder or a file we
// do not overwrite silently stays Tauri's. Fail instead. The reverse is fine:
// mipmap-anydpi-v26 carries the adaptive icon, which the template has no
// equivalent for.
const missing = []
for (const density of readdirSync(target).filter(isMipmap)) {
  if (!densities.includes(density)) {
    missing.push(`${density}/`)
    continue
  }
  const replacements = readdirSync(join(source, density))
  for (const name of readdirSync(join(target, density))) {
    if (!replacements.includes(name)) missing.push(`${density}/${name}`)
  }
}
if (missing.length > 0) {
  throw new Error(`${source} has no replacement for: ${missing.join(', ')}`)
}

let copied = 0
for (const density of densities) {
  mkdirSync(join(target, density), { recursive: true })
  for (const name of readdirSync(join(source, density))) {
    copyFileSync(join(source, density, name), join(target, density, name))
    copied += 1
  }
}
console.log(`Copied ${copied} launcher icons into ${target}`)
