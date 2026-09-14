// Optional browser test: supply an installed Playwright module via PLAYWRIGHT_MODULE.
import {createRequire} from 'node:module';
import {createServer} from 'node:http';
import {readFile,mkdir} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const {chromium}=createRequire(import.meta.url)(process.env.PLAYWRIGHT_MODULE??'playwright');
const dist=path.resolve('dist');
const server=createServer(async(req,res)=>{
 try {const file=path.join(dist,req.url==='/'?'index.html':req.url.split('?')[0]);if(!file.startsWith(dist+path.sep))throw Error('path');const body=await readFile(file);res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html');res.end(body);}catch{res.statusCode=404;res.end();}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const browser=await chromium.launch({headless:true});
try {
 const page=await browser.newPage({viewport:{width:400,height:620},locale:'de-DE'});
 const errors=[];page.on('pageerror',e=>errors.push(e));
 await page.addInitScript(()=>{
  const callbacks=new Map(),listeners=new Map();let next=0;
  const saved=JSON.parse(localStorage.getItem('fixture-settings')??'{"autostart":true,"sync":false,"language":"system"}');
  const snapshot={settings:saved,locale:saved.language==='en'?'en':'de',scanning:false,message:{key:'error.partial',params:{succeeded:'1',failed:'1'},causes:[{key:'error.invalid_setting'}]},devices:[{id:'left',name:'Elgato Key Light Links',info:{productName:'Key Light',serialNumber:'ABC'},host:'127.0.0.1',port:9123,state:{on:1,brightness:3,temperature:280},error:null},{id:'right',name:'Elgato Key Light Rechts',info:{},host:'127.0.0.1',port:9124,state:null,error:{key:'error.technical',params:{details:'Connection refused'}}}]};
  window.fixtureCalls=[];
  window.fixtureLongName=()=>{snapshot.devices[0].name="ElgatoKeyLight"+"x".repeat(100);emit();};
  const emit=()=>{for(const [id,listener] of listeners)if(listener.event==='lights-changed')callbacks.get(listener.handler)?.({id,event:listener.event,payload:structuredClone(snapshot)});};
  window.__TAURI_INTERNALS__={transformCallback(fn){const id=++next;callbacks.set(id,fn);return id;},unregisterCallback(id){callbacks.delete(id);},async invoke(command,args){
   if(command==='plugin:event|listen'){const id=++next;listeners.set(id,args);return id;}
   if(command==='plugin:event|unlisten'){listeners.delete(args.eventId);return;}
   if(command==='snapshot')return structuredClone(snapshot);
   window.fixtureCalls.push({command,args});
   if(command==='update_settings'){Object.assign(snapshot.settings,args);snapshot.locale=snapshot.settings.language==='en'?'en':'de';localStorage.setItem('fixture-settings',JSON.stringify(snapshot.settings));emit();return;}
   if(command==='rename_light')throw {key:'error.name'};
   throw Error('Unexpected fixture command '+command);
  }};
  window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(){}};
 });
 await page.goto(`http://127.0.0.1:${server.address().port}`);
 await page.getByRole('button',{name:'Einstellungen',exact:true}).click();
 await page.getByLabel('Sprache / Language').selectOption('en');
 await page.getByRole('checkbox',{name:/Start automatically in the panel/}).waitFor();
 assert.match(await page.getByRole('alert').textContent(),/1 succeeded; 1 failed: Invalid light setting/);
 assert.equal(await page.locator('html').getAttribute('lang'),'en');
 await page.getByLabel('Sprache / Language').selectOption('de');
 await page.getByRole('checkbox',{name:/Automatisch im Panel starten/}).waitFor();
 assert.match(await page.getByRole('alert').textContent(),/1 erfolgreich; 1 fehlgeschlagen: Ungültige Lampeneinstellung/);
 const calls=await page.evaluate(()=>window.fixtureCalls);
 assert.deepEqual(calls,[{command:'update_settings',args:{language:'en'}},{command:'update_settings',args:{language:'de'}}]);
 assert.equal(await page.getByRole('checkbox').nth(0).isChecked(),true);assert.equal(await page.getByRole('checkbox').nth(1).isChecked(),false);
 await mkdir('artifacts',{recursive:true});await page.screenshot({path:'artifacts/settings-de.png',fullPage:true});
 await page.getByRole('button',{name:'Zurück',exact:true}).click();
 await page.getByRole('button',{name:'Details zu Elgato Key Light Links',exact:true}).click();
 await page.getByLabel('Lampenname',{exact:true}).fill('Invalid fixture name');await page.getByRole('button',{name:'Namen speichern',exact:true}).click();
 await page.getByRole('alert').filter({hasText:'Der Name muss'}).waitFor();
 await page.getByRole('button',{name:'Einstellungen',exact:true}).click();await page.getByLabel('Sprache / Language').selectOption('en');
 await page.getByRole('alert').filter({hasText:'Name must contain'}).waitFor();
 await page.reload();await page.getByRole('button',{name:'Settings',exact:true}).click();
 assert.equal(await page.getByLabel('Sprache / Language').inputValue(),'en');
 await page.screenshot({path:'artifacts/settings-en.png',fullPage:true});
 await page.getByLabel('Sprache / Language').selectOption('system');await page.getByRole('checkbox',{name:/Automatisch im Panel starten/}).waitFor();
 assert.equal(await page.locator('html').getAttribute('lang'),'de');
 const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth);assert.equal(overflow,false);
 await page.getByRole('button',{name:'Zurück',exact:true}).click();
 await page.evaluate(()=>window.fixtureLongName());
 await page.getByRole('button',{name:/Details zu ElgatoKeyLight/}).click();
 for (const name of ['Zurück','Lampen suchen','Einstellungen']) {
  const button=page.getByRole('button',{name,exact:true});
  const bounds=await button.boundingBox();
  assert.ok(bounds && bounds.x>=0 && bounds.x+bounds.width<=400, name+' stays in viewport');
 }
 assert.equal(await page.locator('header').evaluate(el=>el.scrollWidth<=el.clientWidth),true);
 await page.screenshot({path:'artifacts/details-long-name.png',fullPage:true});
 assert.deepEqual(errors,[]);
 console.log('PASS: immediate DE/EN/system switching, saved selection, translated existing status and command errors, isolated language payloads, unchanged checkbox settings, 400px layout');
}finally{await browser.close();await new Promise(r=>server.close(r));}
