import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const en=JSON.parse(readFileSync(new URL('../src/locales/en.json',import.meta.url)));
const de=JSON.parse(readFileSync(new URL('../src/locales/de.json',import.meta.url)));
const tokens=s=>[...s.matchAll(/\{([^{}]+)\}/g)].map(m=>m[1]).sort();
test('German and English catalogs have identical complete keys and placeholders',()=>{
 assert.deepEqual(Object.keys(de).sort(),Object.keys(en).sort());
 for(const key of Object.keys(en)) {assert.ok(en[key].trim());assert.ok(de[key].trim());assert.deepEqual(tokens(de[key]),tokens(en[key]),key);}
});
test('every literal translation key used by Rust and React exists',()=>{
 for(const file of ['src/App.tsx','src-tauri/src/i18n.rs','src-tauri/src/lib.rs','src-tauri/src/lights.rs']) {
  const source=readFileSync(new URL('../'+file,import.meta.url),'utf8');
  for(const match of source.matchAll(/(?:\btr\(|text\(locale,\s*|Message::new\()"([^"]+)"/g))assert.ok(en[match[1]],`${file}: ${match[1]}`);
 }
});
