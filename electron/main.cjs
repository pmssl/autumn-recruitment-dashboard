const { app, BrowserWindow, Menu, Tray, nativeImage, ipcMain, shell, dialog } = require('electron');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { createStore } = require('./store.cjs');

app.setName('秋招看板');
let window, tray, quitting = false;
const page = pathToFileURL(path.join(__dirname, '../dist/index.html')).href;

function showWindow() {
  if (!window) return;
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
}

function openLink(url) {
  try {
    if (['https:', 'http:'].includes(new URL(url).protocol)) shell.openExternal(url).catch(() => {});
  } catch {}
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on('second-instance', showWindow);
  app.on('before-quit', () => { quitting = true; });
  app.on('activate', showWindow);
  app.whenReady().then(async () => {
    app.setAppUserModelId('cn.autumn.recruitment.dashboard');
    const store = createStore(app.getPath('userData'));
    const trusted = event => {
      if (event.sender !== window.webContents || event.senderFrame !== window.webContents.mainFrame || event.senderFrame.url !== page) throw new Error('非法来源');
    };
    ipcMain.handle('data:load', event => { trusted(event); return store.read(); });
    ipcMain.handle('data:save', (event, data) => { trusted(event); store.write(data); });
    Menu.setApplicationMenu(null);
    window = new BrowserWindow({
      width: 1240, height: 820, minWidth: 760, minHeight: 560,
      title: '秋招看板', backgroundColor: '#f7f8fa', show: false,
      webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, nodeIntegration: false, sandbox: true }
    });
    window.webContents.setWindowOpenHandler(({ url }) => { openLink(url); return { action: 'deny' }; });
    window.webContents.on('will-navigate', (event, url) => { if (url !== page) { event.preventDefault(); openLink(url); } });
    window.webContents.session.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
    window.on('close', event => { if (!quitting) { event.preventDefault(); window.hide(); } });
    window.on('session-end', () => { quitting = true; });
    // A small blue progress-chain icon, generated locally without remote assets.
    const pixels = Buffer.alloc(32 * 32 * 4);
    for (let y = 0; y < 32; y++) for (let x = 0; x < 32; x++) {
      const i = (y * 32 + x) * 4;
      const white = (Math.abs(y - 16) < 2 && x > 6 && x < 26) || [8, 16, 24].some(cx => (x-cx)**2 + (y-16)**2 < 12);
      pixels[i] = white ? 255 : 217; pixels[i+1] = white ? 255 : 85; pixels[i+2] = white ? 255 : 40; pixels[i+3] = 255;
    }
    tray = new Tray(nativeImage.createFromBitmap(pixels, { width: 32, height: 32 }));
    tray.setToolTip('秋招看板 · 点击打开，右键退出');
    const startupArgs = ['--hidden'];
    function refreshMenu() {
      tray.setContextMenu(Menu.buildFromTemplate([
        { label: '打开秋招看板', click: showWindow },
        { label: '开机启动', type: 'checkbox', enabled: app.isPackaged,
          checked: app.getLoginItemSettings({ path: process.execPath, args: startupArgs }).openAtLogin,
          click: item => { app.setLoginItemSettings({ openAtLogin: item.checked, path: process.execPath, args: startupArgs }); refreshMenu(); } },
        { label: '打开数据文件夹', click: () => shell.openPath(app.getPath('userData')) },
        { type: 'separator' },
        { label: '退出', click: () => app.quit() }
      ]));
    }
    refreshMenu();
    tray.on('click', showWindow);
    window.once('ready-to-show', () => { if (!process.argv.includes('--hidden')) showWindow(); });
    await window.loadFile(path.join(__dirname, '../dist/index.html'));
  }).catch(error => { dialog.showErrorBox('启动失败', error.message); app.quit(); });
}
