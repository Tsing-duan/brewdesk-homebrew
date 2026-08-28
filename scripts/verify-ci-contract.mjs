import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.argv[2] ?? path.join(import.meta.dirname, ".."));
const workflowDirectory = path.join(root, ".github", "workflows");
const checkoutRef = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const setupNodeRef = "actions/setup-node@820762786026740c76f36085b0efc47a31fe5020";

function fail(message) {
  console.error(message);
  process.exitCode = 1;
}

if (!fs.existsSync(workflowDirectory)) {
  fail(`Missing workflow directory: ${workflowDirectory}`);
  process.exit();
}

const workflowFiles = fs
  .readdirSync(workflowDirectory, { withFileTypes: true })
  .filter((entry) => entry.isFile() && /\.ya?ml$/i.test(entry.name))
  .map((entry) => path.join(workflowDirectory, entry.name))
  .sort();

if (workflowFiles.length === 0) {
  fail("At least one GitHub Actions workflow is required.");
  process.exit();
}

let checkoutCount = 0;
let setupNodeCount = 0;
let allText = "";
const immutableAction = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(?:\/[^\s@]+)?@[0-9a-f]{40}$/;

for (const file of workflowFiles) {
  const text = fs.readFileSync(file, "utf8");
  allText += `\n${text}`;
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(/^(\s*)(?:-\s*)?uses:\s*([^\s#]+)/);
    if (!match) continue;
    const indentation = match[1].length;
    const reference = match[2];
    const block = [line];
    for (let next = index + 1; next < lines.length; next += 1) {
      const candidate = lines[next];
      if (candidate.trim() === "" || candidate.trimStart().startsWith("#")) {
        block.push(candidate);
        continue;
      }
      const candidateIndentation = candidate.match(/^\s*/)[0].length;
      if (candidateIndentation < indentation) break;
      block.push(candidate);
    }
    const blockText = block.join("\n");

    if (!immutableAction.test(reference)) {
      fail(`${path.basename(file)} uses a mutable or invalid Action reference: ${reference}`);
    }
    if (reference.startsWith("actions/checkout@")) {
      checkoutCount += 1;
      if (reference !== checkoutRef) fail(`checkout must use the approved SHA: ${reference}`);
      if (!/persist-credentials:\s*false\b/.test(blockText)) {
        fail("Every checkout step must set persist-credentials: false.");
      }
    }
    if (reference.startsWith("actions/setup-node@")) {
      setupNodeCount += 1;
      if (reference !== setupNodeRef) fail(`setup-node must use the approved SHA: ${reference}`);
      if (!/node-version:\s*["']?24["']?\b/.test(blockText)) {
        fail("Every setup-node step must set node-version: 24.");
      }
      if (!/package-manager-cache:\s*false\b/.test(blockText)) {
        fail("Every setup-node step must set package-manager-cache: false.");
      }
    }
  }
}

if (checkoutCount === 0) fail("The workflows must use actions/checkout.");
if (setupNodeCount === 0) fail("The workflows must use actions/setup-node.");
if (!/^permissions:\s*$[\s\S]*?^\s+contents:\s*read\s*$/m.test(allText)) {
  fail("Top-level permissions must include contents: read.");
}
if (!/runs-on:\s*macos-26\b/.test(allText)) fail("The CI contract must use macos-26.");
if (!/^\s{2}secret-scan:\s*$[\s\S]*?fetch-depth:\s*0\b/m.test(allText)) {
  fail("The secret-scan job must checkout full history with fetch-depth: 0.");
}
if (/8\.30\.1/.test(allText) || !/GITLEAKS_VERSION:\s*["']?8\.30\.0["']?/.test(allText)) {
  fail("The workflow must fix Gitleaks at 8.30.0 and must not mention 8.30.1.");
}
if (!/scripts\/gitleaks-canary\.py/.test(allText)) fail("The workflow must run the Gitleaks Canary.");
if (!/node_modules\/\.bin\/tauri/.test(allText)) fail("The workflow must verify the project-local Tauri CLI.");
if (!/PUBLIC_BUILD_ROOT/.test(allText) || !/CARGO_TARGET_DIR/.test(allText)) {
  fail("The workflow must direct web/native and Cargo outputs outside the repository.");
}
if (!/cargo install cargo-audit --version 0\.22\.2 --locked/.test(allText)) {
  fail("The workflow must install exact cargo-audit 0.22.2 with --locked.");
}

if (process.exitCode) process.exit(process.exitCode);
console.log(`Verified ${workflowFiles.length} workflow file(s); all ${checkoutCount + setupNodeCount} required Action references use approved full SHAs.`);
