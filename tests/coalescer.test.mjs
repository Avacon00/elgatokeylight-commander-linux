import test from 'node:test';
import assert from 'node:assert/strict';
import { createCoalescer } from '../.test-build/coalescer.js';
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
test('rapid slider values merge without losing another property',async()=>{
 const sent=[];const queue=createCoalescer(async p=>{sent.push(p);},()=>{},assert.fail,10);
 queue.push({brightness:10});queue.push({temperature:200});queue.push({brightness:75});await queue.flush();
 assert.deepEqual(sent,[{brightness:75,temperature:200}]);
});
test('slow device has one active request and only the newest pending values',async()=>{
 const sent=[];let release;let active=0;let maxActive=0;
 const queue=createCoalescer(async p=>{active++;maxActive=Math.max(active,maxActive);sent.push(p);if(sent.length===1)await new Promise(r=>release=r);active--;},()=>{},assert.fail,5);
 queue.push({brightness:10});const done=queue.flush();
 queue.push({brightness:25});queue.push({brightness:100});queue.push({on:1});await sleep(15);assert.equal(sent.length,1);release();await done;
 assert.equal(maxActive,1);assert.deepEqual(sent,[{brightness:10},{brightness:100,on:1}]);
});
test('failed request does not stall newer input',async()=>{
 const errors=[];const sent=[];const queue=createCoalescer(async p=>{sent.push(p);if(sent.length===1)throw new Error('offline');},()=>{},e=>errors.push(e),10);
 queue.push({on:1});await queue.flush();queue.push({on:0});await queue.flush();assert.equal(errors.length,1);assert.equal(sent.length,2);
});
test('flush on unmount preserves the final slider value',async()=>{
 const sent=[];const queue=createCoalescer(async p=>{sent.push(p);},()=>{},assert.fail,1000);
 queue.push({brightness:60});await queue.flush();assert.deepEqual(sent,[{brightness:60}]);
});
