import assert from 'node:assert/strict';
import test from 'node:test';

import {
  formatCount,
  formatDate,
  formatDateTime,
  formatDuration,
  formatDurationShort,
  formatPercent,
  formatTotal,
  formatVariation,
} from '../../site/js/format.js';

test('a duration shows only the units it needs', () => {
  assert.equal(formatDuration(1513.45), '25min 13s');
  assert.equal(formatDuration(3723), '1h 02min 03s');
  assert.equal(formatDuration(3600), '1h 00min 00s');
  assert.equal(formatDuration(600), '10min 00s');
  assert.equal(formatDuration(45), '45s');
  assert.equal(formatDuration(0), '0s');
});

test('a duration drops the fraction of a second, to agree with the form that shows the milliseconds', () => {
  assert.equal(formatDuration(1472.9), '24min 32s');
  assert.equal(formatDuration(1472.9, { milliseconds: true }), '24min 32s 900ms');
  assert.equal(formatDuration(59.6), '59s');
  assert.equal(formatDuration(3599.5), '59min 59s');
});

test('a duration can show its milliseconds', () => {
  assert.equal(formatDuration(1513.45, { milliseconds: true }), '25min 13s 450ms');
  assert.equal(formatDuration(1513.29, { milliseconds: true }), '25min 13s 290ms');
  assert.equal(formatDuration(45, { milliseconds: true }), '45s 000ms');
});

test('the short form of a duration drops what is zero', () => {
  assert.equal(formatDurationShort(0), '0');
  assert.equal(formatDurationShort(30), '30s');
  assert.equal(formatDurationShort(60), '1min');
  assert.equal(formatDurationShort(90), '1min 30s');
  assert.equal(formatDurationShort(1500), '25min');
  assert.equal(formatDurationShort(3600), '1h');
  assert.equal(formatDurationShort(5400), '1h30');
  assert.equal(formatDurationShort(1513.5), '25min 13,5s');
});

test('a variation carries its sign', () => {
  assert.equal(formatVariation(0.3), '+0,30s');
  assert.equal(formatVariation(-0.3), '−0,30s');
  assert.equal(formatVariation(150), '+2min 30s');
  assert.equal(formatVariation(-65.2), '−1min 05s');
  assert.equal(formatVariation(59.999), '+1min 00s');
});

test('a variation too small to show has no sign', () => {
  assert.equal(formatVariation(0), '0,00s');
  assert.equal(formatVariation(-0.001), '0,00s');
});

test('a percentage carries its sign', () => {
  assert.equal(formatPercent(0.0198), '+0,02%');
  assert.equal(formatPercent(-0.0198), '−0,02%');
  assert.equal(formatPercent(46.666), '+46,67%');
  assert.equal(formatPercent(0), '0,00%');
  assert.equal(formatPercent(-0.001), '0,00%');
});

test('dates are shown in the time of São Paulo', () => {
  const alreadyNewYearInUtc = Date.parse('2024-01-01T02:30:00Z');

  assert.equal(formatDate(alreadyNewYearInUtc), '31/12/2023');
  assert.equal(formatDateTime(Date.parse('2026-10-13T13:20:00Z')), '13/10/2026 10:20');
});

test('counts use the Brazilian thousands separator', () => {
  assert.equal(formatCount(1523), '1.523');
  assert.equal(formatCount(42), '42');
});

test('a long total is shown in days', () => {
  assert.equal(formatTotal(37 * 3600 + 12 * 60), '37h 12min');
  assert.equal(formatTotal(26 * 86400 + 18 * 3600 + 1200), '26 dias 18h');
});
