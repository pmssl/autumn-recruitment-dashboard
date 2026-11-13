import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const output = resolve(root, "dist");
rmSync(output, { recursive: true, force: true });
mkdirSync(output, { recursive: true });
for (const file of ["index.html", "main.js"]) {
  copyFileSync(resolve(root, "public", file), resolve(output, file));
}
mkdirSync(resolve(output, "fonts"), { recursive: true });
for (const file of ["NotoSansSC-Regular.otf", "OFL.txt"]) {
  copyFileSync(resolve(root, "public", "fonts", file), resolve(output, "fonts", file));
}
const result = spawnSync("wasm-pack", [
  "build", resolve(root, "frontend"), "--release", "--target", "web",
  "--out-dir", resolve(output, "pkg"),
], { stdio: "inherit" });
if (result.status !== 0) process.exit(result.status || 1);
