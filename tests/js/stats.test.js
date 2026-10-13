import assert from 'node:assert/strict';
import test from 'node:test';

import { groupByPeriod, lastYears, prepare, summarize } from '../../site/js/stats.js';

const DATA = [
  { id: 'a', title: 'A', publishedAt: '2023-12-31T12:00:00Z', duration: 100, mean: 100 },
  // Already 2024 in UTC, but still the last night of 2023 in São Paulo.
  { id: 'b', title: 'B', publishedAt: '2024-01-01T02:30:00Z', duration: 200, mean: 150 },
  { id: 'c', title: 'C', publishedAt: '2024-03-10T12:00:00Z', duration: 600, mean: 300 },
];

test('each video gets its time, its link and the variation it caused', () => {
  const [a, b, c] = prepare(DATA);

  assert.deepEqual(a, {
    id: 'a',
    title: 'A',
    time: Date.parse('2023-12-31T12:00:00Z'),
    duration: 100,
    mean: 100,
    variation: 0,
    variationPercent: 0,
    url: 'https://www.youtube.com/watch?v=a',
  });
  assert.deepEqual([b.variation, b.variationPercent], [50, 50]);
  assert.deepEqual([c.variation, c.variationPercent], [150, 100]);
});

test('a range keeps the videos of the last years', () => {
  const videos = prepare(DATA);

  assert.deepEqual(lastYears(videos, 1, Date.parse('2024-06-01T00:00:00Z')).map((v) => v.id), ['a', 'b', 'c']);
  assert.deepEqual(lastYears(videos, 1, Date.parse('2025-02-01T00:00:00Z')).map((v) => v.id), ['c']);
  assert.deepEqual(lastYears(videos, null, Date.parse('2030-01-01T00:00:00Z')).map((v) => v.id), ['a', 'b', 'c']);
});

test('a range does not change the variation of a video', () => {
  const [c] = lastYears(prepare(DATA), 1, Date.parse('2025-02-01T00:00:00Z'));

  assert.equal(c.variation, 150);
});

test('groups the videos by the year they were published in São Paulo', () => {
  const years = groupByPeriod(prepare(DATA), 'year');

  assert.deepEqual(
    years.map(({ key, label, count, total, mean }) => ({ key, label, count, total, mean })),
    [
      { key: '2023', label: '2023', count: 2, total: 300, mean: 150 },
      { key: '2024', label: '2024', count: 1, total: 600, mean: 600 },
    ],
  );
});

test('groups by month, keeping the months without videos', () => {
  const months = groupByPeriod(prepare(DATA), 'month');

  assert.deepEqual(
    months.map(({ key, label, count, mean }) => ({ key, label, count, mean })),
    [
      { key: '2023-12', label: 'dez/23', count: 2, mean: 150 },
      { key: '2024-01', label: 'jan/24', count: 0, mean: null },
      { key: '2024-02', label: 'fev/24', count: 0, mean: null },
      { key: '2024-03', label: 'mar/24', count: 1, mean: 600 },
    ],
  );
});

test('summarizes the videos', () => {
  const videos = prepare(DATA);

  assert.deepEqual(summarize(videos), { count: 3, total: 900, longest: videos[2] });
});

test('copes with no videos at all', () => {
  assert.deepEqual(prepare([]), []);
  assert.deepEqual(groupByPeriod([], 'year'), []);
  assert.deepEqual(summarize([]), { count: 0, total: 0, longest: null });
});
