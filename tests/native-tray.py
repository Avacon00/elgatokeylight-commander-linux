#!/usr/bin/env python3
"""Exercise the real COSMIC DBusMenu with loopback lamps and isolated app config.
Usage: /usr/bin/python3 tests/native-tray.py /absolute/path/to/keylight-commander-linux
Never sends control requests to discovered physical lights.
"""
import json, os, pathlib, subprocess, sys, tempfile, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from gi.repository import Gio, GLib

bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
def call(dest,path,interface,method,args=None):
 return bus.call_sync(dest,path,interface,method,args,None,Gio.DBusCallFlags.NONE,5000,None).unpack()
def prop(dest,path,interface,name):return call(dest,path,'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',(interface,name)))[0]
def registered():return set(prop('org.kde.StatusNotifierWatcher','/StatusNotifierWatcher','org.kde.StatusNotifierWatcher','RegisteredStatusNotifierItems'))
def wait_for(check,seconds=20):
 deadline=time.monotonic()+seconds
 while time.monotonic()<deadline:
  result=check()
  if result:return result
  time.sleep(.15)
 raise AssertionError('Timed out waiting for condition')
def fake(serial):
 state={'on':0,'brightness':35,'temperature':200};requests=[];fail=[False]
 info={'serialNumber':serial,'macAddress':serial,'displayName':serial,'productName':'Key Light'}
 class Handler(BaseHTTPRequestHandler):
  def log_message(self,*args):pass
  def do_GET(self):
   if fail[0]:self.send_error(503);return
   self.respond(info if self.path.endswith('accessory-info') else {'numberOfLights':1,'lights':[state]})
  def do_PUT(self):
   data=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
   requests.append(data)
   if 'lights' in data:state.update(data['lights'][0])
   else:info.update(data)
   self.respond({})
  def respond(self,value):
   data=json.dumps(value).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
 server=ThreadingHTTPServer(('127.0.0.1',0),Handler);threading.Thread(target=server.serve_forever,daemon=True).start()
 return server,state,info,requests,fail

language=next((arg.split('=',1)[1] for arg in sys.argv if arg.startswith('--language=')),'en')
identifier=next((arg.split('=',1)[1] for arg in sys.argv if arg.startswith('--identifier=')),'io.github.keylight-commander-linux')
translations=json.loads((pathlib.Path(__file__).resolve().parents[1]/'src'/'locales'/f'{language}.json').read_text())
def tr(key):return translations[key]
a=fake('Fixture A');b=fake('Fixture B');binary=str(pathlib.Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix='keylight-native-') as temp:
 root=pathlib.Path(temp);config=root/identifier;config.mkdir()
 devices=[{'id':'serialNumber:'+fixture[2]['serialNumber'].lower(),'name':fixture[2]['displayName'],'host':'127.0.0.1','port':fixture[0].server_port,'info':fixture[2]} for fixture in [a,b]]
 (config/'settings.json').write_text(json.dumps({'settings':{'autostart':False,'sync':True,'initialized':True,'language':language},'devices':devices}))
 env={**os.environ,'XDG_CONFIG_HOME':temp};baseline=registered()
 log=open('/tmp/keylight-native-test.log','w');process=subprocess.Popen([binary,'--background'],env=env,stdout=log,stderr=log)
 try:
  item=wait_for(lambda:next(iter(registered()-baseline),None))
  dest,_,path=item.partition('/');path='/'+path if path else '/StatusNotifierItem'
  menu_path=prop(dest,path,'org.kde.StatusNotifierItem','Menu')
  icon=prop(dest,path,'org.kde.StatusNotifierItem','IconName')
  assert icon,'Tray must publish an icon'
  def tree():return call(dest,menu_path,'com.canonical.dbusmenu','GetLayout',GLib.Variant('(iias)',(0,-1,[])))[1]
  def flatten(node):
   ident,props,children=node
   return [(ident,props)]+[item for child in children for item in flatten(child)]
  def find(label,within=None):return next(((i,p) for i,p in flatten(within or tree()) if label in p.get('label','')),None)
  wait_for(lambda:find('Fixture A · '+tr('off')))
  def event(ident):call(dest,menu_path,'com.canonical.dbusmenu','Event',GLib.Variant('(isvu)',(ident,'clicked',GLib.Variant('s',''),0)))
  def light_sub(name):return next(child for child in tree()[2] if name in child[1].get('label',''))
  def direct_find(label,node):return next(((i,p) for i,p,children in node[2] if label==p.get('label','')),None)
  assert light_sub('Fixture A')[1]['label']=='Fixture A · '+tr('off')
  assert all(not children for _,_,children in light_sub('Fixture A')[2]),'COSMIC presets must not use third-level submenus'
  status=find(tr('brightness')+':',light_sub('Fixture A'))
  assert status and not status[1].get('enabled',True) and '5000 K' in status[1]['label']
  event(find(tr('turn_on'),light_sub('Fixture A'))[0]);wait_for(lambda:a[1]['on']==1)
  assert b[1]['on']==0,'Individual panel action must ignore window sync'
  wait_for(lambda:find('Fixture A · '+tr('on')))
  event(direct_find('75%',light_sub('Fixture A'))[0]);wait_for(lambda:a[1]['brightness']==75)
  assert a[1]['temperature']==200 and a[1]['on']==1
  event(direct_find('3000 K',light_sub('Fixture B'))[0]);wait_for(lambda:b[1]['temperature']==333)
  assert b[1]['brightness']==35 and b[1]['on']==0
  # Group actions are safe only while every registered device is a fixture.
  wait_for(lambda:find(tr('scan')) and find(tr('scan'))[1].get('enabled',True))
  saved=json.loads((config/'settings.json').read_text())
  groups_tested=all(d['host']=='127.0.0.1' for d in saved['devices'])
  if groups_tested:
   event(find(tr('all_on'))[0]);wait_for(lambda:a[1]['on']==1 and b[1]['on']==1)
   event(find(tr('all_off'))[0]);wait_for(lambda:a[1]['on']==0 and b[1]['on']==0)
  event(find(tr('open_controls'))[0])
  if '--screenshot' in sys.argv:
   time.sleep(2)
   subprocess.run(['cosmic-screenshot','--interactive=false','--notify=false','--save-dir',str(pathlib.Path('artifacts').resolve())],check=True,timeout=15)
  event(find(tr('settings'))[0])
  b[4][0]=True
  wait_for(lambda:light_sub('Fixture B')[1]['label']=='Fixture B · '+tr('offline'))
  offline=light_sub('Fixture B')
  assert find(tr('brightness')+':',offline) is None,'Offline lamp must not display stale state'
  assert all(not props.get('enabled',True) for _,props,_ in offline[2] if props.get('type')!='separator'),'Offline controls must be disabled'
  second=subprocess.run([binary],env=env,timeout=10,stdout=log,stderr=log);assert second.returncode==0
  assert len(registered()-baseline)==1,'Second launch must not duplicate the icon'
  event(find(tr('quit'))[0]);assert process.wait(timeout=10)==0
  wait_for(lambda:not (registered()-baseline))
  print(language+': PASS: icon registration, flat COSMIC presets, individual targeting, offline state/disabled controls, window-sync isolation, Open/Settings dispatch, single instance, Quit/unregister')
  print('Groups: '+('PASS' if groups_tested else 'skipped in native test (physical lights discovered); covered by Rust simulations'))
 finally:
  if process.poll() is None:process.terminate();process.wait(timeout=10)
  log.close()
  for fixture in [a,b]:fixture[0].shutdown()
