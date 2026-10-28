#!/usr/bin/env node
// Read-only project statistics. No build, install, or upload is performed.
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
const options = { json: false, out: null };
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--json') options.json = true;
  else if (args[i] === '--out' && args[i + 1] && !args[i + 1].startsWith('--')) options.out = args[++i];
  else if (args[i] === '--help') {
    console.log('npm run metrics [-- --json] [-- --out metrics-report/report.json]\n统计当前工作区源码、配置、文档及已有安装包；不会自动构建。');
    process.exit(0);
  } else { console.error(`未知参数或缺少参数值：${args[i]}`); process.exit(1); }
}

const excludedDirs = new Set(['.git', 'node_modules', 'dist', 'release', 'target', 'backup', 'sync-output', 'metrics-report', '.dev-data', '.venv', '__pycache__']);
const sourceExtensions = new Set(['.ts', '.tsx', '.js', '.jsx', '.cjs', '.mjs', '.css', '.scss', '.html', '.vue', '.svelte', '.rs', '.slint', '.c', '.h', '.cpp', '.hpp', '.py', '.ps1', '.sh']);
const configExtensions = new Set(['.json', '.toml', '.yaml', '.yml']);
const documentExtensions = new Set(['.md', '.txt', '.rst']);
const lockFiles = new Set(['package-lock.json', 'Cargo.lock', 'pnpm-lock.yaml', 'yarn.lock']);
const personalScripts = new Set(['scripts/prepare-excel-sync.cjs', 'scripts/apply-excel-sync.ps1']);
const warnings = [];
const relative = value => path.relative(root, value).split(path.sep).join('/');
function walk(directory) {
  if (!fs.existsSync(directory)) return [];
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) return [];
    return entry.isDirectory() ? walk(full) : entry.isFile() ? [full] : [];
  });
}
function fallbackWalk(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    if (entry.isSymbolicLink() || excludedDirs.has(entry.name)) return [];
    const full = path.join(directory, entry.name);
    return entry.isDirectory() ? fallbackWalk(full) : entry.isFile() ? [relative(full)] : [];
  });
}
let candidates;
let fileSelection = 'git tracked + untracked, respecting .gitignore';
try {
  candidates = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).split('\0').filter(Boolean);
} catch {
  fileSelection = 'filesystem scan with built-in exclusions';
  warnings.push('Git 不可用，改用目录扫描；只应用内置排除规则。');
  candidates = fallbackWalk(root);
}
const files = [];
for (const name of new Set(candidates)) {
  if (name.split('/').some(part => excludedDirs.has(part)) || personalScripts.has(name)) continue;
  const base = path.basename(name);
  if (lockFiles.has(base) || base.startsWith('.env') || base.endsWith('.tsbuildinfo')) continue;
  const ext = path.extname(name).toLowerCase();
  const category = sourceExtensions.has(ext) ? 'source' : configExtensions.has(ext) || base === '.gitignore' ? 'config' : documentExtensions.has(ext) ? 'docs' : null;
  if (!category) continue;
  const full = path.join(root, name);
  if (!fs.existsSync(full) || !fs.lstatSync(full).isFile()) continue;
  const buffer = fs.readFileSync(full);
  if (buffer.includes(0)) { warnings.push(`跳过疑似二进制文件：${name}`); continue; }
  const content = buffer.toString('utf8');
  const lines = content ? content.split(/\r\n|\n|\r/) : [];
  if (lines.length && lines.at(-1) === '') lines.pop();
  const group = name.startsWith('native/') ? 'Rust / Slint 新版' : /^(src|electron)\//.test(name) ? 'React / Electron 旧版' : name.startsWith('scripts/') ? '项目工具' : '其他';
  files.push({ path: name, category, group, extension: ext || '(none)', bytes: buffer.length, lines: lines.length, nonEmptyLines: lines.filter(line => line.trim()).length });
}
files.sort((a, b) => a.path.localeCompare(b.path));
const sum = items => items.reduce((s, f) => ({ files: s.files + 1, bytes: s.bytes + f.bytes, lines: s.lines + f.lines, nonEmptyLines: s.nonEmptyLines + f.nonEmptyLines }), { files: 0, bytes: 0, lines: 0, nonEmptyLines: 0 });
const summarizeBy = (items, key) => Object.fromEntries([...new Set(items.map(f => f[key]))].map(value => [value, sum(items.filter(f => f[key] === value))]));
const sources = files.filter(f => f.category === 'source');
const artifact = (file, kind) => {
  const stat = fs.statSync(file);
  return { path: relative(file), kind, bytes: stat.size, MB: Number((stat.size / 1e6).toFixed(2)), MiB: Number((stat.size / 1048576).toFixed(2)), modifiedAt: stat.mtime.toISOString() };
};
const artifacts = [];
const roots = ['release', 'native/dist', 'native/release', 'native/target/release/bundle'];
for (const dir of roots) {
  for (const file of walk(path.join(root, dir))) {
    const name = relative(file);
    if (name.includes('/win-unpacked/') || /__uninstaller|\.blockmap$/i.test(name)) continue;
    const extension = /\.(exe|msi|dmg|pkg|appimage|deb|rpm|zip|tar\.gz)$/i.test(name);
    if (!extension) continue;
    const kind = /\.(zip|tar\.gz)$/i.test(name) ? '压缩分发包' : /\.(exe|appimage)$/i.test(name) && !/setup|install/i.test(path.basename(name)) ? '独立可执行程序（不等于安装包）' : '安装包';
    artifacts.push(artifact(file, kind));
  }
}
for (const name of ['native/target/release/autumn-native.exe', 'native/target/release/autumn-native']) {
  const full = path.join(root, name);
  if (fs.existsSync(full) && fs.statSync(full).isFile()) artifacts.push(artifact(full, '原生可执行程序'));
}
artifacts.sort((a, b) => a.path.localeCompare(b.path, undefined, { numeric: true }));
const buildDirectories = ['dist', 'release/win-unpacked'].filter(name => fs.existsSync(path.join(root, name))).map(name => {
  const entries = walk(path.join(root, name));
  return { path: name, files: entries.length, bytes: entries.reduce((total, file) => total + fs.statSync(file).size, 0) };
});
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const currentInstaller = artifacts.find(a => path.basename(a.path) === `Autumn-Dashboard-Setup-${pkg.version}.exe`) || null;
if (!currentInstaller) warnings.push(`未找到与 package.json ${pkg.version} 匹配的 Electron 安装包。`);
const report = {
  generatedAt: new Date().toISOString(), projectVersion: pkg.version, fileSelection,
  notes: ['行数为物理行数，包含注释；非空行数不等于有效逻辑代码行数。', '统计当前磁盘文件，包含尚未提交的新代码；依赖、锁文件、构建产物及个人数据不计入源码。', '安装包仅测量已有文件，不自动构建；版本匹配不代表已包含当前代码修改。', 'MB = 1,000,000 字节；MiB = 1,048,576 字节。'],
  source: { total: sum(sources), byGroup: summarizeBy(sources, 'group'), byExtension: summarizeBy(sources, 'extension') },
  config: sum(files.filter(f => f.category === 'config')), docs: sum(files.filter(f => f.category === 'docs')),
  currentInstaller, artifacts, buildDirectories, files, warnings,
};
if (options.out) {
  const destination = path.resolve(root, options.out);
  const reportRoot = path.join(root, 'metrics-report') + path.sep;
  if (!destination.startsWith(reportRoot) || !destination.endsWith('.json')) throw new Error('--out 必须是 metrics-report/ 下的 .json 文件，避免覆盖项目文件');
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, JSON.stringify(report, null, 2) + '\n');
}
if (options.json) console.log(JSON.stringify(report, null, 2));
else {
  console.log(`\n秋招看板 · 项目体积统计（Electron ${pkg.version}）`);
  console.log('源码行数包含注释；不含依赖、锁文件、个人数据和构建产物。\n');
  console.table(Object.entries(report.source.byGroup).map(([group, s]) => ({ 分组: group, 文件数: s.files, 总行数: s.lines, 非空行数: s.nonEmptyLines, '源码 KiB': (s.bytes / 1024).toFixed(2) })));
  console.log(`源码合计：${report.source.total.files} 文件 / ${report.source.total.lines} 行 / ${(report.source.total.bytes / 1024).toFixed(2)} KiB`);
  console.log(`配置：${report.config.files} 文件 / ${report.config.lines} 行；文档：${report.docs.files} 文件 / ${report.docs.lines} 行（单独统计）\n`);
  console.table(artifacts.map(a => ({ 文件: a.path, 类型: a.kind, MB: a.MB, MiB: a.MiB })));
  if (currentInstaller) console.log(`当前版本安装包：${currentInstaller.path}，${currentInstaller.MB} MB / ${currentInstaller.MiB} MiB`);
  if (!artifacts.some(a => a.path.startsWith('native/'))) console.log('Rust / Slint 新版：尚无可测量的构建产物。');
  console.table(buildDirectories.map(d => ({ 构建目录: d.path, 文件数: d.files, MB: (d.bytes / 1e6).toFixed(2), MiB: (d.bytes / 1048576).toFixed(2) })));
  console.log('注意：现有安装包可能早于当前代码；本命令不会自动构建。');
  warnings.forEach(w => console.log(`提示：${w}`));
  if (options.out) console.log(`JSON 报告已保存：${options.out}`);
}
