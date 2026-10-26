// Run with Electron; all test data stays in a separate temporary profile.
const { app } = require('electron');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const assert = require('node:assert/strict');
const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'autumn-desktop-smoke-'));
app.setPath('userData', profile);
process.argv.push('--hidden');
const timeout = setTimeout(() => { console.error('Desktop smoke test timed out'); app.exit(1); }, 30000);
app.once('browser-window-created', (_event, window) => {
  window.webContents.once('did-finish-load', async () => {
    try {
      const run = code => window.webContents.executeJavaScript(code);
      for (let i = 0; i < 100; i++) {
        if (await run("document.body.innerText.includes('新增投递')")) break;
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      assert.equal(await run("typeof window.desktop.save"), 'function');
      assert.equal(await run("document.body.innerText.includes('新增投递')"), true);
      const currentDate = new Date();
      const calendarDate = `${currentDate.getFullYear()}-${String(currentDate.getMonth()+1).padStart(2,'0')}-${String(currentDate.getDate()).padStart(2,'0')}`;
      const data = [{ id:'smoke', company:'桌面持久化测试', position:'开发', createdAt:'2026-10-10', updatedAt:'2026-10-10', stages:[{id:'test-applied',applicationId:'smoke',type:'applied',date:calendarDate,title:'投递',status:'completed',sortOrder:10},{id:'test-interview',applicationId:'smoke',type:'interview',date:calendarDate,title:'一面',status:'pending',sortOrder:20}] }];
      await run(`window.desktop.save(${JSON.stringify(data)})`);
      assert.deepEqual(JSON.parse(fs.readFileSync(path.join(profile, 'applications.json'), 'utf8')), data);
      const loaded = new Promise(resolve => window.webContents.once('did-finish-load', resolve));
      window.reload();
      await loaded;
      for (let i = 0; i < 100; i++) {
        if (await run("document.body.innerText.includes('桌面持久化测试')")) break;
        await new Promise(resolve => setTimeout(resolve, 100));
      }
      assert.equal(await run("document.body.innerText.includes('桌面持久化测试')"), true);
      const pause = () => new Promise(resolve => setTimeout(resolve, 200));
      async function click(selector) {
        const point = await run(`(() => {const e=document.querySelector(${JSON.stringify(selector)}); if(!e)throw Error('Missing: '+${JSON.stringify(selector)}); e.scrollIntoView({block:'center'}); const r=e.getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})()`);
        window.webContents.sendInputEvent({type:'mouseMove',...point});
        window.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...point});
        window.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...point});
        await pause();
      }
      await click('nav button:nth-child(3)');
      assert.equal(await run(`document.querySelector('[data-date="${calendarDate}"]').classList.contains('has-events')`),true);
      assert.equal(await run("document.querySelectorAll('.calendar-event').length"),2);
      await click('.calendar-pending input');
      assert.equal(await run("document.querySelectorAll('.calendar-event').length"),1);
      await click('[aria-label="下个月"]');
      assert.equal(await run("document.querySelectorAll('.calendar-event').length"),0);
      await click('.calendar-month-controls > button:last-child');
      assert.equal(await run("document.querySelectorAll('.calendar-event').length"),1);
      await click('.calendar-event');
      assert.equal(await run("document.querySelector('.drawer h2').textContent"),'桌面持久化测试');
      await click('.drawer-head > button');
      await click('nav button:first-child');
      console.log('PASS: calendar date highlighting, pending filter, month navigation, today and application details');
      await click('.application-heading');
      await click('.v-stage:nth-child(2) .stage-actions button');
      assert.equal(await run("document.querySelector('.modal-head h2')?.textContent"), '编辑进展');
      await click('.modal input');
      assert.equal(await run("document.activeElement === document.querySelector('.modal input')"), true);
      await run("document.querySelector('.modal input').select()");
      window.webContents.insertText('二面回归测试');
      await pause();
      await click('.modal .actions .primary');
      assert.equal(await run("document.querySelector('.modal') === null"), true);
      assert.equal(JSON.parse(fs.readFileSync(path.join(profile,'applications.json'),'utf8'))[0].stages[1].title,'二面回归测试');
      console.log('PASS: detail edit click, input focus, save to disk');
      window.close();
      assert.equal(window.isDestroyed(), false);
      assert.equal(window.isVisible(), false);
      console.log('PASS: preload, rendered UI, persistence after reload, close-to-tray');
      clearTimeout(timeout);
      app.quit();
    } catch (error) { console.error(error); app.exit(1); }
  });
});
require('./main.cjs');
