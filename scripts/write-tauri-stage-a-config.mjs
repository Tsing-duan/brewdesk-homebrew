import fs from "node:fs";
import path from "node:path";

const [buildRootArgument, projectRootArgument, expectedIdentifier] = process.argv.slice(2);
if (!buildRootArgument || !projectRootArgument || !expectedIdentifier) {
  throw new Error("Usage: write-tauri-stage-a-config.mjs BUILD_ROOT PROJECT_ROOT BUNDLE_IDENTIFIER");
}

const buildRoot = fs.realpathSync(buildRootArgument);
const projectRoot = fs.realpathSync(projectRootArgument);
if (buildRoot === projectRoot || buildRoot.startsWith(`${projectRoot}${path.sep}`)) {
  throw new Error("Build root must resolve outside the repository.");
}

const sourceConfigPath = path.join(projectRoot, "src-tauri", "tauri.conf.json");
const sourceConfig = JSON.parse(fs.readFileSync(sourceConfigPath, "utf8"));
if (sourceConfig.identifier !== expectedIdentifier) {
  throw new Error("Expected Bundle Identifier does not match tauri.conf.json.");
}

const configuredIcons = sourceConfig.bundle.icon ?? [];
if (configuredIcons.length === 0) {
  throw new Error("No user-approved application icons are configured.");
}
const icons = configuredIcons.map((icon) => path.join(projectRoot, "src-tauri", icon));
for (const icon of icons) {
  if (!fs.statSync(icon, { throwIfNoEntry: false })?.isFile()) {
    throw new Error(`Missing approved application icon: ${icon}`);
  }
}

const override = {
  identifier: expectedIdentifier,
  build: {
    beforeBuildCommand: "",
    frontendDist: path.join(buildRoot, "web"),
  },
  bundle: {
    externalBin: [
      path.join(buildRoot, "native", "brewdesk-translate"),
      path.join(buildRoot, "native", "brewdesk-translation-setup"),
    ],
    icon: icons,
  },
};

const output = path.join(buildRoot, "tauri-stage-a.override.json");
const temporary = path.join(buildRoot, `.tauri-stage-a.override.${process.pid}.tmp`);
fs.writeFileSync(temporary, `${JSON.stringify(override, null, 2)}\n`, { encoding: "utf8", mode: 0o600, flag: "wx" });
fs.renameSync(temporary, output);
fs.chmodSync(output, 0o600);
console.log(output);
