// Scales and axis ticks for the charts.

const YEAR_MS = 365.25 * 86_400_000;
const MONTHS = ['jan', 'fev', 'mar', 'abr', 'mai', 'jun', 'jul', 'ago', 'set', 'out', 'nov', 'dez'];
// Ticks are placed at midnight in São Paulo, taken as UTC-3: an hour of error does not show on an axis.
const UTC_OFFSET_HOURS = 3;
const DURATION_STEPS = [
  0.1, 0.2, 0.5, 1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10_800, 21_600, 43_200, 86_400,
];

/** A function mapping `domain` onto `range`, with an `invert` that maps back. */
export function linear([domainStart, domainEnd], [rangeStart, rangeEnd]) {
  const domainSpan = domainEnd - domainStart;
  const rangeSpan = rangeEnd - rangeStart;
  const scale = (value) =>
    domainSpan ? rangeStart + ((value - domainStart) / domainSpan) * rangeSpan : (rangeStart + rangeEnd) / 2;
  scale.invert = (position) => (rangeSpan ? domainStart + ((position - rangeStart) / rangeSpan) * domainSpan : domainStart);
  return scale;
}

/** At most `maxTicks` durations between `min` and `max` seconds, at the finest round step that fits. */
export function durationTicks(min, max, maxTicks) {
  const fits = (step) => Math.floor(max / step + 1e-9) - Math.ceil(min / step - 1e-9) + 1 <= maxTicks;
  const step = DURATION_STEPS.find(fits) ?? 86_400 * Math.ceil((max - min) / 86_400 / maxTicks);
  const first = Math.ceil(min / step - 1e-9);
  const last = Math.floor(max / step + 1e-9);
  return Array.from({ length: Math.max(last - first + 1, 0) }, (_, index) => Math.round((first + index) * step * 10) / 10);
}

/** At most `maxTicks` moments between `start` and `end`: the start of years, or of months when the span is short. */
export function timeTicks(start, end, maxTicks) {
  const yearly = end - start >= 2 * YEAR_MS;
  const firstYear = new Date(start).getUTCFullYear();
  const lastYear = new Date(end).getUTCFullYear();
  // Months are counted from year zero, so that steps fall on the same months every year.
  const [first, last] = yearly ? [firstYear, lastYear] : [firstYear * 12, lastYear * 12 + 11];

  for (const step of yearly ? [1, 2, 5, 10, 20, 50, 100] : [1, 2, 3, 6]) {
    const ticks = [];
    for (let index = Math.ceil(first / step) * step; index <= last; index += step) {
      const [year, month] = yearly ? [index, 0] : [Math.floor(index / 12), index % 12];
      const time = Date.UTC(year, month, 1, UTC_OFFSET_HOURS);
      const label = yearly ? String(year) : `${MONTHS[month]}/${String(year).slice(-2)}`;
      if (time >= start && time <= end) ticks.push({ time, label });
    }
    if (ticks.length <= maxTicks) return ticks;
  }
  return [];
}

/** The index of the value in the sorted `values` that is nearest to `target`, or -1 when there is none. */
export function nearestIndex(values, target) {
  if (!values.length) return -1;
  let low = 0;
  let high = values.length - 1;
  while (low < high) {
    const middle = (low + high) >> 1;
    if (values[middle] < target) low = middle + 1;
    else high = middle;
  }
  const previous = low - 1;
  return previous >= 0 && target - values[previous] <= values[low] - target ? previous : low;
}
