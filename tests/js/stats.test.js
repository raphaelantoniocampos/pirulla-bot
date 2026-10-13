import assert from 'node:assert/strict';
import test from 'node:test';

import { groupByPeriod, lastYears, ofKinds, prepare, summarize, withRate } from '../../site/js/stats.js';

const DATA = [
  { id: 'a', title: 'A', publishedAt: '2023-12-31T12:00:00Z', duration: 100, kind: 'video' },
  // Already 2024 in UTC, but still the last night of 2023 in São Paulo.
  { id: 'b', title: 'B', publishedAt: '2024-01-01T02:30:00Z', duration: 200, kind: 'live' },
  { id: 'c', title: 'C', publishedAt: '2024-03-10T12:00:00Z', duration: 600, kind: 'short' },
];

const ids = (videos) => videos.map((video) => video.id);

test('each video gets its time, its kind and its link', () => {
  const [a, b, c] = prepare(DATA);

  assert.deepEqual(a, {
    id: 'a',
    title: 'A',
    time: Date.parse('2023-12-31T12:00:00Z'),
    duration: 100,
    kind: 'video',
    url: 'https://www.youtube.com/watch?v=a',
  });
  assert.deepEqual([b.kind, c.kind], ['live', 'short']);
});

test('a video without a kind is a regular video', () => {
  const [video] = prepare([{ id: 'a', title: 'A', publishedAt: '2023-12-31T12:00:00Z', duration: 100 }]);

  assert.equal(video.kind, 'video');
});

test('regular videos always count; lives and shorts count when asked for', () => {
  const videos = prepare(DATA);

  assert.deepEqual(ids(ofKinds(videos, { lives: false, shorts: false })), ['a']);
  assert.deepEqual(ids(ofKinds(videos, { lives: true, shorts: false })), ['a', 'b']);
  assert.deepEqual(ids(ofKinds(videos, { lives: false, shorts: true })), ['a', 'c']);
  assert.deepEqual(ids(ofKinds(videos, { lives: true, shorts: true })), ['a', 'b', 'c']);
});

test('the rate is the average duration up to each video, with the variation it caused', () => {
  const rated = withRate(prepare(DATA));

  assert.deepEqual(rated.map((video) => video.mean), [100, 150, 300]);
  assert.deepEqual(rated.map((video) => video.variation), [0, 50, 150]);
  assert.deepEqual(rated.map((video) => video.variationPercent), [0, 50, 100]);
});

test('the rate counts only the videos it is given', () => {
  const rated = withRate(ofKinds(prepare(DATA), { lives: false, shorts: true }));

  assert.deepEqual(ids(rated), ['a', 'c']);
  assert.deepEqual(rated.map((video) => video.mean), [100, 350]);
  assert.deepEqual(rated.map((video) => video.variation), [0, 250]);
});

test('the rate is rounded to hundredths of a second', () => {
  const durations = [10, 10, 11].map((duration, index) => ({
    id: String(index),
    title: '',
    publishedAt: `2024-01-0${index + 1}T12:00:00Z`,
    duration,
  }));

  assert.equal(withRate(prepare(durations)).at(-1).mean, 10.33);
});

test('rating does not change the videos it is given', () => {
  const videos = prepare(DATA);

  withRate(videos);

  assert.equal('mean' in videos[0], false);
});

test('a range keeps the videos of the last years', () => {
  const videos = prepare(DATA);

  assert.deepEqual(ids(lastYears(videos, 1, Date.parse('2024-06-01T00:00:00Z'))), ['a', 'b', 'c']);
  assert.deepEqual(ids(lastYears(videos, 1, Date.parse('2025-02-01T00:00:00Z'))), ['c']);
  assert.deepEqual(ids(lastYears(videos, null, Date.parse('2030-01-01T00:00:00Z'))), ['a', 'b', 'c']);
});

test('a range does not change the variation of a video', () => {
  const [c] = lastYears(withRate(prepare(DATA)), 1, Date.parse('2025-02-01T00:00:00Z'));

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
  assert.deepEqual(withRate([]), []);
  assert.deepEqual(groupByPeriod([], 'year'), []);
  assert.deepEqual(summarize([]), { count: 0, total: 0, longest: null });
});
