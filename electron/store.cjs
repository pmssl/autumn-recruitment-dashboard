const fs = require('node:fs');
const path = require('node:path');

function validate(data) {
  if (!Array.isArray(data) || data.some(a => !a || typeof a.id !== 'string' || typeof a.company !== 'string' || typeof a.position !== 'string' || !Array.isArray(a.stages) || a.stages.some(s => !s || s.applicationId !== a.id || typeof s.id !== 'string' || typeof s.title !== 'string' || !['completed', 'pending', 'failed'].includes(s.status)))) {
    throw new Error('投递数据格式不正确');
  }
  return data;
}

function createStore(directory) {
  const file = path.join(directory, 'applications.json');
  return {
    file,
    read() {
      if (!fs.existsSync(file)) return null;
      return validate(JSON.parse(fs.readFileSync(file, 'utf8')));
    },
    write(data) {
      validate(data);
      fs.mkdirSync(directory, { recursive: true });
      const temporary = file + '.tmp';
      fs.writeFileSync(temporary, JSON.stringify(data, null, 2), 'utf8');
      if (fs.existsSync(file)) fs.copyFileSync(file, file + '.bak');
      fs.renameSync(temporary, file);
    }
  };
}
module.exports = { createStore, validate };
