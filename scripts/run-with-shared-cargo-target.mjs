import { execFileSync, spawn } from "node:child_process";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const [command, ...args] = process.argv.slice(2);

if (!command) {
  console.error("Usage: node scripts/run-with-shared-cargo-target.mjs <command> [args...]");
  process.exit(1);
}

function resolveGitCommonDir() {
  const commonDirRaw = execFileSync("git", ["rev-parse", "--git-common-dir"], {
    cwd: repoRoot,
    encoding: "utf8",
  }).trim();

  return isAbsolute(commonDirRaw) ? commonDirRaw : resolve(repoRoot, commonDirRaw);
}

const gitCommonDir = resolveGitCommonDir();
const sharedTargetDir = join(dirname(gitCommonDir), "src-tauri", "target");
const needsShell = process.platform === "win32" && /\.(cmd|bat)$/i.test(command);

const child = spawn(command, args, {
  cwd: repoRoot,
  env: {
    ...process.env,
    CARGO_TARGET_DIR: sharedTargetDir,
  },
  shell: needsShell,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
