import { execFileSync, spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
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
const localEnv = loadDotEnv(join(repoRoot, ".env"));

function loadDotEnv(path) {
  if (!existsSync(path)) {
    return {};
  }

  const entries = {};
  for (const line of readFileSync(path, "utf8").split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }

    const separatorIndex = trimmed.indexOf("=");
    if (separatorIndex <= 0) {
      continue;
    }

    const key = trimmed.slice(0, separatorIndex).trim();
    let value = trimmed.slice(separatorIndex + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    entries[key] = value;
  }

  return entries;
}

const child = spawn(command, args, {
  cwd: repoRoot,
  env: {
    ...process.env,
    ...localEnv,
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
