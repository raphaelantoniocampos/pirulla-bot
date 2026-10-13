// Text shown to the reader: durations, variations, dates and counts, in Brazilian Portuguese.

const TIME_ZONE = 'America/Sao_Paulo';
const MINUS = '−';

const dateParts = new Intl.DateTimeFormat('en-GB', {
  timeZone: TIME_ZONE,
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
});
const count = new Intl.NumberFormat('pt-BR');

const pad = (number) => String(number).padStart(2, '0');
const decimal = (number, digits) => number.toFixed(digits).replace('.', ',');
const sign = (value, isZero) => (isZero ? '' : value > 0 ? '+' : MINUS);

function partsOf(timestamp) {
  const parts = {};
  for (const { type, value } of dateParts.formatToParts(timestamp)) parts[type] = value;
  return parts;
}

/** "25min 13s"; with `milliseconds`, "25min 13s 450ms". Without them, the fraction of a second is dropped. */
export function formatDuration(seconds, { milliseconds = false } = {}) {
  const total = Math.round(Math.abs(seconds) * 1000);
  const hours = Math.floor(total / 3_600_000);
  const minutes = Math.floor((total % 3_600_000) / 60_000);
  const wholeSeconds = Math.floor((total % 60_000) / 1000);

  let text = `${wholeSeconds}s`;
  if (hours) text = `${hours}h ${pad(minutes)}min ${pad(wholeSeconds)}s`;
  else if (minutes) text = `${minutes}min ${pad(wholeSeconds)}s`;
  return milliseconds ? `${text} ${String(total % 1000).padStart(3, '0')}ms` : text;
}

/** The shortest readable form, for axis ticks: "45s", "25min", "1h30". */
export function formatDurationShort(seconds) {
  if (seconds === 0) return '0';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = Math.round((seconds % 60) * 10) / 10;

  if (hours) return minutes ? `${hours}h${pad(minutes)}` : `${hours}h`;
  const restText = rest ? `${String(rest).replace('.', ',')}s` : '';
  if (minutes) return restText ? `${minutes}min ${restText}` : `${minutes}min`;
  return restText;
}

/** A change in a duration, with its sign: "+0,30s", "−1min 05s". */
export function formatVariation(seconds) {
  const size = Math.round(Math.abs(seconds) * 100) / 100;
  const text = size < 60 ? `${decimal(size, 2)}s` : formatDuration(Math.round(size));
  return sign(seconds, size === 0) + text;
}

/** "+0,02%" */
export function formatPercent(percent) {
  const size = Math.abs(percent);
  return `${sign(percent, size < 0.005)}${decimal(size, 2)}%`;
}

/** "31/12/2023", in the time of São Paulo. */
export function formatDate(timestamp) {
  const { day, month, year } = partsOf(timestamp);
  return `${day}/${month}/${year}`;
}

/** "13/10/2026 10:20", in the time of São Paulo. */
export function formatDateTime(timestamp) {
  const { day, month, year, hour, minute } = partsOf(timestamp);
  return `${day}/${month}/${year} ${hour}:${minute}`;
}

/** "1.523" */
export function formatCount(number) {
  return count.format(number);
}

/** A total of many durations: "37h 12min", or "26 dias 18h" from two days on. */
export function formatTotal(seconds) {
  const hours = Math.floor(seconds / 3600);
  if (hours < 48) return `${hours}h ${pad(Math.floor((seconds % 3600) / 60))}min`;
  return `${formatCount(Math.floor(hours / 24))} dias ${hours % 24}h`;
}

/** What a video is, when it is not a regular one: "live" or "short". */
export function formatKind(kind) {
  return kind === 'live' || kind === 'short' ? kind : '';
}
