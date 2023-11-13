// @ts-check
import { family, GLIBC, MUSL } from "detect-libc"
import { exec } from 'child_process'
import util from 'util'

const execa = util.promisify(exec)
const platform = process.platform
const arch = process.arch

let libcFamily
family().then((fam) => {
  libcFamily = fam
})

let libc
if (platform === "win32") {
  libc = "-msvc"
} else {
  libc = libcFamily === GLIBC ? "-gnu" : libcFamily === MUSL ? "-musl" : ""
}

const pkg = `@tailcallhq/core-${platform}-${arch}${libc}`

try {
  // @ts-ignore
  const { stdout, stderr } = await execa(`npm install ${pkg}@${version} --no-save`)
  stderr ? console.log(stderr) : console.log(`Successfully installed optional dependency: ${pkg}`, stdout)
  // Install Scarf SDK as part of the post-install process
  const { scarfStdout, scarfStderr } = await execa('npm i --save @scarf/scarf')
  scarfStderr ? console.log(scarfStderr) : console.log('Scarf SDK installed:', scarfStdout)
} catch (error) {
  console.error(`Failed to install optional dependency: ${pkg}`, error.stderr)
}
