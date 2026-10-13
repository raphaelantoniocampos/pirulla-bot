// The text of a post announcing a video, to be posted on X by hand.

import { formatDuration, formatPercent, formatVariation } from './format.js';

/** The most a post can weigh, as X counts it. */
export const MAX_TWEET_LENGTH = 280;

/** What X counts for a link, whatever its real length. */
const LINK_LENGTH = 23;
const ELLIPSIS = '…';
const MINUS = '−';

/**
 * The length of `text` as X counts it: every link counts as 23 characters, and
 * the characters outside the Latin ranges, such as emoji, count as two.
 *
 * X also turns bare domains such as `g1.com` into links. Words that look like
 * one count as themselves plus a link, so that the result is never too short.
 */
export function weightedLength(text) {
  let length = 0;
  for (const word of text.split(/(\s+)/)) {
    if (/^https?:\/\//.test(word)) {
      length += LINK_LENGTH;
      continue;
    }
    for (const character of word) length += characterWeight(character);
    if (/[\p{L}\p{N}]\.\p{L}/u.test(word)) length += LINK_LENGTH;
  }
  return length;
}

function characterWeight(character) {
  const code = character.codePointAt(0);
  const light =
    code <= 0x10ff || (code >= 0x2000 && code <= 0x200d) || (code >= 0x2010 && code <= 0x201f) || (code >= 0x2032 && code <= 0x2037);
  return light ? 1 : 2;
}

/** Cuts `text` at the longest length that weighs at most `maxLength`, ending it with an ellipsis. */
function truncate(text, maxLength) {
  if (weightedLength(text) <= maxLength) return text;
  const characters = [...text];
  for (let end = characters.length - 1; end > 0; end -= 1) {
    const cut = characters.slice(0, end).join('').trimEnd() + ELLIPSIS;
    if (weightedLength(cut) <= maxLength) return cut;
  }
  return '';
}

/**
 * Writes the post announcing `video`, with a link to the website at `siteUrl`.
 * The title is cut when the post would not fit in {@link MAX_TWEET_LENGTH}.
 */
export function composeTweet(video, siteUrl) {
  const variation = formatVariation(video.variation);
  const trend = variation.startsWith(MINUS) ? '📉' : '📈';
  const write = (title) =>
    [
      '🚨 VÍDEO NOVO 🚨',
      `🎥 ${title}`,
      `🕒 Duração: ${formatDuration(video.duration)}`,
      `🔗 ${video.url}`,
      '',
      `⏳ 1 Pirulla: ${formatDuration(video.mean, { milliseconds: true })}`,
      `📊 Variação: ${trend} ${variation} (${formatPercent(video.variationPercent)})`,
      `🌐 Gráficos: ${siteUrl}`,
    ].join('\n');

  const room = MAX_TWEET_LENGTH - weightedLength(write(''));
  return write(truncate(video.title, room));
}

/** The address that opens X with `text` ready to be posted. */
export function intentUrl(text) {
  return `https://x.com/intent/post?text=${encodeURIComponent(text)}`;
}
