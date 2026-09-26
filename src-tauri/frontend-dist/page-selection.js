/** Physical PDF pages, numbered from 1. Ranges are inclusive. */
export class PageSelectionError extends Error {
 constructor(key, value = '') { super(key); this.key = key; this.value = value; }
}
export function planPageSelection(input, pageCount, mode = 'selected') {
 if (!Number.isSafeInteger(pageCount) || pageCount < 1) throw new PageSelectionError('splitNoPages');
 if (!['selected', 'each', 'ranges'].includes(mode)) throw new PageSelectionError('splitInvalidMode');
 const text = String(input || '').trim().replace(/[–—−]/g, '-');
 if (!text) throw new PageSelectionError('splitEmpty');
 if (text.length > 10000) throw new PageSelectionError('splitTooLong');
 let expanded = 0;
 const groups = text.split(',').map(part => {
  const match = part.trim().match(/^(\d+)\s*(?:-\s*(\d+))?$/);
  if (!match) throw new PageSelectionError('splitInvalidRange', part.trim());
  const start = Number(match[1]), end = Number(match[2] || match[1]);
  if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 1 || end > pageCount)
   throw new PageSelectionError('splitOutOfBounds', pageCount);
  if (start > end) throw new PageSelectionError('splitReverseRange', part.trim());
  expanded += end-start+1;
  if (expanded > 100000) throw new PageSelectionError('splitTooLong');
  return Array.from({length:end-start+1}, (_, i) => start+i);
 });
 const pages = [...new Set(groups.flat())].sort((a,b) => a-b);
 return { pages, groups: mode === 'ranges' ? groups : mode === 'each' ? pages.map(n => [n]) : [pages] };
}
