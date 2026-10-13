// Loads the data written by the generator and fills the page with it.

import { createChart, paintPeriods, paintRate, paintVideos } from './js/charts.js';
import {
  formatCount,
  formatDate,
  formatDateTime,
  formatDuration,
  formatKind,
  formatPercent,
  formatTotal,
  formatVariation,
} from './js/format.js';
import { groupByPeriod, lastYears, ofKinds, prepare, summarize, withRate } from './js/stats.js';
import { MAX_TWEET_LENGTH, composeTweet, intentUrl, weightedLength } from './js/tweet.js';

const DATA_URL = 'data.json';
const YEAR_MS = 365.25 * 86_400_000;
/** Spans shorter than this are shown month by month instead of year by year. */
const MONTHLY_SPAN = 2.5 * YEAR_MS;
const FIRST_ROWS = 15;
const MORE_ROWS = 50;

const byId = (id) => document.getElementById(id);

// Titles come from YouTube, so they are always set as text, never as HTML.
function node(tag, className, text) {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text !== undefined) element.textContent = text;
  return element;
}

function link(url, text) {
  const anchor = node('a', '', text);
  anchor.href = url;
  anchor.target = '_blank';
  anchor.rel = 'noopener';
  return anchor;
}

/** "▲ +0,30s (+0,02%)", colored by its direction. */
function variationNode(video) {
  const direction = Math.abs(video.variation) < 0.005 ? 'flat' : video.variation > 0 ? 'up' : 'down';
  const arrow = { up: '▲ ', down: '▼ ', flat: '' }[direction];
  const text = `${formatVariation(video.variation)} (${formatPercent(video.variationPercent)})`;
  return node('span', `variation ${direction}`, arrow + text);
}

/** A table cell. On a narrow screen the rows are stacked and `label` says what the cell holds. */
function cell(className, content, label) {
  const element = node('td', className);
  element.append(...[content].flat());
  if (label) element.dataset.label = label;
  return element;
}

/** The headline and the tiles, for the videos that count. */
function fillSummary(videos) {
  const latest = videos.at(-1);
  if (!latest) {
    byId('rate').textContent = '—';
    byId('rate-ms').textContent = '';
    byId('delta').replaceChildren();
    byId('last-video').textContent = 'Nenhum vídeo entra na cotação com este filtro.';
    for (const tile of ['count', 'total', 'longest']) {
      byId(`tile-${tile}`).textContent = '—';
      byId(`tile-${tile}-note`).replaceChildren();
    }
    return;
  }
  const [rate, milliseconds] = formatDuration(latest.mean, { milliseconds: true }).split(/ (?=\d+ms$)/);
  byId('rate').textContent = rate;
  byId('rate-ms').textContent = milliseconds;
  byId('delta').replaceChildren(variationNode(latest), ' ', node('span', 'context', 'com o último vídeo'));
  byId('last-video').replaceChildren(
    'Último vídeo: ',
    link(latest.url, latest.title),
    ` · ${formatDuration(latest.duration)} · ${formatDate(latest.time)}`,
  );

  const summary = summarize(videos);
  byId('tile-count').textContent = formatCount(summary.count);
  byId('tile-count-note').textContent = `desde ${formatDate(videos[0].time)}`;
  byId('tile-total').textContent = formatTotal(summary.total);
  byId('tile-total-note').textContent = `${formatCount(Math.round(summary.total / 3600))} horas de vídeo`;
  byId('tile-longest').textContent = formatDuration(summary.longest.duration);
  byId('tile-longest-note').replaceChildren(link(summary.longest.url, summary.longest.title));
}

/** The title of a video, linked, with a mark when it is a live or a short. */
function titleNodes(video) {
  const kind = formatKind(video.kind);
  return kind ? [link(video.url, video.title), node('span', 'tag', kind)] : [link(video.url, video.title)];
}

function fillPeriodTable(groups) {
  const rows = [...groups].reverse().map((group) => {
    const row = node('tr');
    row.append(
      cell('', group.label),
      cell('num', formatCount(group.count)),
      cell('num', group.count ? formatDuration(group.mean) : '—'),
      cell('num', group.count ? formatTotal(group.total) : '—'),
    );
    return row;
  });
  byId('period-rows').replaceChildren(...rows);
}

/** The table of videos, most recent first, shown a page at a time. */
function videoTable() {
  const body = byId('video-rows');
  const more = byId('more');
  let videos = [];
  let shown = 0;

  function show(count) {
    for (const video of videos.slice(shown, shown + count)) {
      const row = node('tr');
      row.append(
        cell('date', formatDate(video.time)),
        cell('title', titleNodes(video)),
        cell('num', formatDuration(video.duration), 'Duração'),
        cell('num', formatDuration(video.mean, { milliseconds: true }), 'Cotação'),
        cell('num', variationNode(video)),
      );
      body.append(row);
    }
    shown = Math.min(shown + count, videos.length);
    more.hidden = shown >= videos.length;
    more.textContent = `Mostrar mais (${formatCount(videos.length - shown)} restantes)`;
  }

  more.addEventListener('click', () => show(MORE_ROWS));
  return {
    update(next) {
      videos = [...next].reverse();
      shown = 0;
      body.replaceChildren();
      if (videos.length) return show(FIRST_ROWS);
      const row = node('tr');
      const empty = cell('empty', 'Sem vídeos neste período.');
      empty.colSpan = 5;
      row.append(empty);
      body.append(row);
      more.hidden = true;
    },
  };
}

/**
 * The dialog with the post announcing a video, which can be edited, copied or
 * opened in X. `update(video)` sets the video to announce; without one, the
 * button that opens the dialog is hidden.
 */
function tweetDialog() {
  let video = null;
  const dialog = byId('tweet-dialog');
  const text = byId('tweet-text');
  const count = byId('tweet-count');
  const copy = byId('tweet-copy');
  const open = byId('tweet-open');
  // The address of the page without what comes after its directory.
  const siteUrl = new URL('.', window.location.href).href;

  function refresh() {
    const length = weightedLength(text.value);
    const tooLong = length > MAX_TWEET_LENGTH;
    count.textContent = tooLong
      ? `${length} de ${MAX_TWEET_LENGTH} caracteres: o X não aceita um texto deste tamanho.`
      : `${length} de ${MAX_TWEET_LENGTH} caracteres, na contagem do X.`;
    count.classList.toggle('over', tooLong);
    copy.textContent = 'Copiar texto';
    byId('tweet-post').href = intentUrl(text.value);
  }

  open.addEventListener('click', () => {
    text.value = composeTweet(video, siteUrl);
    refresh();
    dialog.showModal();
  });
  text.addEventListener('input', refresh);
  copy.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(text.value);
      copy.textContent = 'Copiado!';
    } catch {
      // Without access to the clipboard, the text is left selected for the reader to copy.
      text.select();
      copy.textContent = 'Selecionado: copie com Ctrl+C';
    }
  });
  // A click on the backdrop lands on the dialog itself.
  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) dialog.close();
  });
  return {
    update(next) {
      video = next;
      open.hidden = !next;
    },
  };
}

function render(data) {
  const videos = prepare(data.videos);
  if (!videos.length) throw new Error('the data file has no videos');
  const now = Date.parse(data.generatedAt);
  byId('channel-link').href = `https://www.youtube.com/channel/${encodeURIComponent(data.channelId)}`;
  byId('updated').textContent = `Atualizado em ${formatDateTime(now)}.`;

  const tweet = tweetDialog();
  const rate = createChart(byId('rate-chart'), { label: 'Gráfico da cotação ao longo do tempo', paint: paintRate });
  const durations = createChart(byId('videos-chart'), {
    label: 'Gráfico da duração de cada vídeo',
    paint: paintVideos,
    height: 420,
  });
  const periods = createChart(byId('period-chart'), { label: 'Gráfico da duração média por período', paint: paintPeriods });
  const table = videoTable();

  // What counts scopes the whole page; the period, the charts and the table below it.
  function show() {
    const counted = withRate(ofKinds(videos, { lives: byId('count-lives').checked, shorts: byId('count-shorts').checked }));
    fillSummary(counted);
    tweet.update(counted.at(-1) ?? null);

    const range = document.querySelector('input[name="range"]:checked').value;
    const selected = lastYears(counted, range === 'all' ? null : Number(range), now);
    const span = selected.length ? selected.at(-1).time - selected[0].time : 0;
    const period = span < MONTHLY_SPAN ? 'month' : 'year';
    const groups = groupByPeriod(selected, period);

    rate.update(selected);
    durations.update(selected);
    periods.update(groups);
    byId('period-title').textContent = `Duração média por ${period === 'year' ? 'ano' : 'mês'}`;
    fillPeriodTable(groups);
    table.update(selected);
  }

  for (const input of document.querySelectorAll('.filters input')) input.addEventListener('change', show);
  show();
}

async function main() {
  try {
    const response = await fetch(DATA_URL, { cache: 'no-cache' });
    if (!response.ok) throw new Error(`${DATA_URL} answered ${response.status}`);
    render(await response.json());
  } catch (error) {
    console.error(error);
    const status = byId('status');
    status.textContent = 'Não foi possível carregar os dados agora. Tente recarregar a página em alguns minutos.';
    status.hidden = false;
  }
}

main();
