const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { createStore } = require('./store.cjs');
test('保存后重启读取、上一版本备份、拒绝非法数据', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'autumn-store-'));
  const store = createStore(dir);
  assert.equal(store.read(), null);
  const data = [{ id: 'a', company: '测试公司', position: '开发', stages: [{ id: 's', applicationId: 'a', title: '投递', status: 'completed' }] }];
  store.write(data);
  assert.deepEqual(createStore(dir).read(), data);
  store.write([]);
  assert.deepEqual(store.read(), []);
  assert.deepEqual(JSON.parse(fs.readFileSync(store.file + '.bak', 'utf8')), data);
  assert.throws(() => store.write([{ ...data[0], stages: [{ ...data[0].stages[0], applicationId: 'other' }] }]));
  assert.deepEqual(store.read(), []);
});
test('损坏数据报错，不能静默覆盖', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'autumn-store-'));
  const store = createStore(dir);
  fs.writeFileSync(store.file, 'broken');
  assert.throws(() => store.read());
  assert.equal(fs.readFileSync(store.file, 'utf8'), 'broken');
});
