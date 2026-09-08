import { copyFileSync, existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

// `tauri ios init` renders the Xcode project from the CLI's own template, which
// carries Tauri's placeholder AppIcon. Nothing in the CLI copies
// src-tauri/icons/ios into the generated asset catalog -- `tauri icon` writes
// there directly, and only when the project already exists -- so a build from a
// clean checkout, which is every CI build, ships the placeholder. Copy the
// committed icons over it after init.
const source = 'src-tauri/icons/ios'
const target = 'src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset'

if (!existsSync(target)) {
  throw new Error(`${target} does not exist; run \`tauri ios init\` first`)
}

const icons = readdirSync(source).filter((name) => name.endsWith('.png'))
// The catalog's Contents.json maps fixed filenames, so a name we do not
// overwrite silently stays Tauri's. Fail instead.
const missing = readdirSync(target)
  .filter((name) => name.endsWith('.png'))
  .filter((name) => !icons.includes(name))
if (missing.length > 0) {
  throw new Error(`${source} has no replacement for: ${missing.join(', ')}`)
}

for (const name of icons) copyFileSync(join(source, name), join(target, name))
console.log(`Copied ${icons.length} app icons into ${target}`)
