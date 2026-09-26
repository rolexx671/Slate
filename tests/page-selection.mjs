import assert from 'node:assert/strict';
import { planPageSelection as plan, PageSelectionError } from '../src-tauri/frontend-dist/page-selection.js';
assert.deepEqual(plan('1-3, 5, 7-10',10).groups, [[1,2,3,5,7,8,9,10]]);
assert.deepEqual(plan('3, 1-2, 2',3).groups, [[1,2,3]]);
assert.deepEqual(plan('1–2, 4',4,'each').groups, [[1],[2],[4]]);
assert.deepEqual(plan('1-2, 4, 2-3',4,'ranges').groups, [[1,2],[4],[2,3]]);
assert.deepEqual(plan('1',1).groups, [[1]]);
for (const input of ['', '0', '1,', ',1', '4', '3-1', '-1', '1.5', '1-9', 'a', '1--2', '999999999999999999999']) {
 assert.throws(()=>plan(input,3),PageSelectionError, input);
}
assert.throws(()=>plan('1-100001',100001),PageSelectionError);
assert.throws(()=>plan('1',0),PageSelectionError);
assert.throws(()=>plan('1',3,'invalid'),PageSelectionError);
console.log('Page selection: ranges, modes, source order, duplicates and invalid input passed.');
