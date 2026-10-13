import assert from 'node:assert/strict';
import test from 'node:test';

import { MAX_TWEET_LENGTH, composeTweet, intentUrl, weightedLength } from '../../site/js/tweet.js';

const VIDEO_URL = 'https://www.youtube.com/watch?v=dQw4w9WgXcQ';
const SITE_URL = 'https://example.github.io/pirulla-bot/';

function video(overrides = {}) {
  return {
    title: 'O que é ciência?',
    duration: 1800,
    mean: 1513.75,
    variation: 0.3,
    variationPercent: 0.0198,
    url: VIDEO_URL,
    ...overrides,
  };
}

test('announces a video that raised the average', () => {
  assert.equal(
    composeTweet(video(), SITE_URL),
    [
      '🚨 VÍDEO NOVO 🚨',
      '🎥 O que é ciência?',
      '🕒 Duração: 30min 00s',
      '🔗 https://www.youtube.com/watch?v=dQw4w9WgXcQ',
      '',
      '⏳ 1 Pirulla: 25min 13s 750ms',
      '📊 Variação: 📈 +0,30s (+0,02%)',
      '🌐 Gráficos: https://example.github.io/pirulla-bot/',
    ].join('\n'),
  );
});

test('announces a video that lowered the average', () => {
  const tweet = composeTweet(video({ variation: -2.48, variationPercent: -0.0862 }), SITE_URL);

  assert.equal(tweet.split('\n')[6], '📊 Variação: 📉 −2,48s (−0,09%)');
});

test('an unchanged average is not shown as a fall', () => {
  const tweet = composeTweet(video({ variation: -0.001, variationPercent: -0.00007 }), SITE_URL);

  assert.equal(tweet.split('\n')[6], '📊 Variação: 📈 0,00s (0,00%)');
});

test('counts length the way X does', () => {
  assert.equal(weightedLength('abc'), 3);
  assert.equal(weightedLength('Variação'), 8);
  assert.equal(weightedLength('🚨'), 2);
  assert.equal(weightedLength('…'), 2);
  assert.equal(weightedLength('−2,48s'), 7);
  assert.equal(weightedLength('a\n\nb'), 4);
  assert.equal(weightedLength(VIDEO_URL), 23);
  assert.equal(weightedLength(`🔗 ${VIDEO_URL}\nfim`), 2 + 1 + 23 + 1 + 3);
});

test('a word that X may turn into a link counts as the word plus a link', () => {
  assert.equal(weightedLength('leia em g1.com hoje'), 4 + 1 + 2 + 1 + (6 + 23) + 1 + 4);
  assert.equal(weightedLength('subiu 0.02% hoje'), 16);
  assert.equal(weightedLength('fim. Outra frase'), 16);
});

test('the longest title YouTube allows fits whole in a typical announcement', () => {
  const title = 'A'.repeat(100);

  const tweet = composeTweet(video({ title }), SITE_URL);

  assert.ok(tweet.includes(title));
  assert.ok(weightedLength(tweet) <= MAX_TWEET_LENGTH, `${weightedLength(tweet)} > 280`);
});

test('a title that cannot fit is cut with an ellipsis, and the links are kept', () => {
  const tweet = composeTweet(video({ title: '🧬'.repeat(100) }), SITE_URL);
  const titleLine = tweet.split('\n')[1];

  assert.ok(weightedLength(tweet) <= MAX_TWEET_LENGTH, `${weightedLength(tweet)} > 280`);
  assert.ok(titleLine.startsWith('🎥 🧬🧬') && titleLine.endsWith('🧬…'), titleLine);
  assert.ok(tweet.includes(VIDEO_URL) && tweet.includes(SITE_URL));
  assert.ok(weightedLength(tweet) > MAX_TWEET_LENGTH - 4, 'the title should use the room there is');
});

test('never exceeds the limit, whatever the title and the numbers', () => {
  const titles = [
    'A'.repeat(100),
    '🧬'.repeat(100),
    `${'A'.repeat(93)} g1.com`,
    `Veja em pirulla.com.br e em g1.com ${'é'.repeat(65)}`,
    'a.b '.repeat(25),
    '漢字'.repeat(50),
  ];
  for (const title of titles) {
    const worst = video({ title, duration: 86_400 + 23 * 3600 + 59 * 60 + 59, mean: 9999.99, variation: -5999.99, variationPercent: -37.5 });

    const tweet = composeTweet(worst, SITE_URL);

    assert.ok(weightedLength(tweet) <= MAX_TWEET_LENGTH, `${weightedLength(tweet)} > 280 for ${title}`);
    assert.ok(tweet.includes(VIDEO_URL) && tweet.includes(SITE_URL));
  }
});

test('the link to post on X carries the whole text', () => {
  const text = 'linha 1\nlinha 2 & 100% #pirulla 🚨';

  const url = new URL(intentUrl(text));

  assert.equal(url.origin + url.pathname, 'https://x.com/intent/post');
  assert.equal(url.searchParams.get('text'), text);
});
