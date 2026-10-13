import assert from 'node:assert/strict';
import test from 'node:test';

import { durationTicks, linear, nearestIndex, timeTicks } from '../../site/js/scale.js';

test('a linear scale maps a domain onto a range and back', () => {
  const scale = linear([0, 100], [20, 520]);

  assert.equal(scale(0), 20);
  assert.equal(scale(50), 270);
  assert.equal(scale.invert(270), 50);
});

test('a linear scale can be inverted, as the y axis of a chart is', () => {
  const scale = linear([0, 100], [300, 0]);

  assert.equal(scale(25), 225);
});

test('a scale over a single value puts it in the middle', () => {
  assert.equal(linear([10, 10], [0, 100])(10), 50);
});

test('duration ticks fall on round steps', () => {
  assert.deepEqual(durationTicks(0, 3600, 6), [0, 900, 1800, 2700, 3600]);
  assert.deepEqual(durationTicks(1500, 1530, 6), [1500, 1510, 1520, 1530]);
  assert.deepEqual(durationTicks(0, 14000, 6), [0, 3600, 7200, 10800]);
});

test('duration ticks can be fractions of a second', () => {
  assert.deepEqual(durationTicks(1512.3, 1513.9, 5), [1512.5, 1513, 1513.5]);
});

test('time ticks fall on round years when the span is long', () => {
  const [start, end] = [Date.parse('2006-11-05T12:00:00Z'), Date.parse('2026-10-13T12:00:00Z')];

  assert.deepEqual(timeTicks(start, end, 8).map((tick) => tick.label), ['2010', '2015', '2020', '2025']);
  assert.deepEqual(
    timeTicks(start, end, 12).map((tick) => tick.label),
    ['2008', '2010', '2012', '2014', '2016', '2018', '2020', '2022', '2024', '2026'],
  );
});

test('time ticks fall on months when the span is short', () => {
  const ticks = timeTicks(Date.parse('2025-10-13T12:00:00Z'), Date.parse('2026-10-13T12:00:00Z'), 6);

  assert.deepEqual(ticks.map((tick) => tick.label), ['nov/25', 'jan/26', 'mar/26', 'mai/26', 'jul/26', 'set/26']);
});

test('time ticks are inside the span, in order', () => {
  const [start, end] = [Date.parse('2006-11-05T12:00:00Z'), Date.parse('2026-10-13T12:00:00Z')];
  const times = timeTicks(start, end, 8).map((tick) => tick.time);

  assert.ok(times.every((time) => time >= start && time <= end));
  assert.deepEqual(times, [...times].sort((a, b) => a - b));
});

test('finds the value nearest to a position', () => {
  const values = [0, 10, 20, 30];

  assert.equal(nearestIndex(values, 14), 1);
  assert.equal(nearestIndex(values, 16), 2);
  assert.equal(nearestIndex(values, -5), 0);
  assert.equal(nearestIndex(values, 99), 3);
  assert.equal(nearestIndex([], 5), -1);
});
