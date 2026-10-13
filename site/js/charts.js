// The charts of the page, drawn as SVG without any library.

import {
  formatCount,
  formatDate,
  formatDuration,
  formatDurationShort,
  formatKind,
  formatPercent,
  formatTotal,
  formatVariation,
} from './format.js';
import { durationTicks, linear, nearestIndex, timeTicks } from './scale.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const HEIGHT = 340;
/** On a narrow screen a chart keeps this share of its height. */
const NARROW_HEIGHT_RATIO = 0.82;
/** Below this width a chart drops its direct labels to leave room for the plot. */
const NARROW_WIDTH = 520;
/** How far, in pixels, the pointer can be from a dot and still select it. */
const HOVER_RADIUS = 28;
const TOOLTIP_GAP = 14;

const clamp = (value, min, max) => Math.min(Math.max(value, min), max);

function svgNode(tag, attributes = {}, text) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
  if (text !== undefined) node.textContent = text;
  return node;
}

function htmlNode(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Keeps a chart in `container`: `update(data)` draws it, and it is drawn again
 * whenever the container changes width.
 *
 * `paint(svg, data, size)` draws the marks and returns how the reader points at
 * them: `count`, `indexAt(x, y)`, `show(index)` (which returns where the tooltip
 * goes), `hide()`, `describe(index)` and, optionally, `open(index)`.
 */
export function createChart(container, { label, paint, height = HEIGHT }) {
  const tooltip = htmlNode('div', 'tooltip');
  const empty = htmlNode('p', 'chart-empty', 'Sem vídeos neste período.');
  let data = [];
  let marks = null;
  let current = null;
  let drawnWidth = 0;
  let pointerType = 'mouse';

  tooltip.hidden = true;
  container.tabIndex = 0;
  container.setAttribute('role', 'group');
  container.setAttribute('aria-label', `${label}. Use as setas do teclado para percorrer os valores.`);

  function draw(force) {
    const width = container.clientWidth;
    if (!width || (!force && width === drawnWidth)) return;
    drawnWidth = width;
    current = null;
    tooltip.hidden = true;
    if (!data.length) {
      marks = null;
      container.replaceChildren(empty);
      return;
    }
    const narrow = width < NARROW_WIDTH;
    const size = { width, height: narrow ? Math.round(height * NARROW_HEIGHT_RATIO) : height, narrow };
    const svg = svgNode('svg', { width, height: size.height, viewBox: `0 0 ${width} ${size.height}`, 'aria-hidden': 'true' });
    marks = paint(svg, data, size);
    container.replaceChildren(svg, tooltip);
  }

  function show(index) {
    current = index;
    const anchor = marks.show(index);
    fillTooltip(tooltip, marks.describe(index));
    tooltip.hidden = false;
    placeTooltip(tooltip, anchor, container);
  }

  function hide() {
    current = null;
    tooltip.hidden = true;
    marks?.hide();
  }

  function onPointer(event) {
    pointerType = event.pointerType;
    if (!marks) return;
    const box = container.getBoundingClientRect();
    const index = marks.indexAt(event.clientX - box.left, event.clientY - box.top);
    if (index < 0) hide();
    else if (index !== current) show(index);
  }

  function onKey(event) {
    if (!marks) return;
    const last = marks.count - 1;
    const stride = event.shiftKey ? 10 : 1;
    let next;
    if (event.key === 'ArrowLeft') next = (current ?? last + 1) - stride;
    else if (event.key === 'ArrowRight') next = (current ?? -1) + stride;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = last;
    else if (event.key === 'Escape') return hide();
    else if (event.key === 'Enter' && current !== null) return marks.open?.(current);
    else return;
    event.preventDefault();
    show(clamp(next, 0, last));
  }

  container.addEventListener('pointermove', onPointer);
  container.addEventListener('pointerdown', onPointer);
  // Lifting a finger also "leaves" the chart, but what was tapped must stay on screen until the next tap.
  container.addEventListener('pointerleave', (event) => {
    if (event.pointerType === 'mouse') hide();
  });
  document.addEventListener('pointerdown', (event) => {
    if (!container.contains(event.target)) hide();
  });
  container.addEventListener('blur', hide);
  container.addEventListener('keydown', onKey);
  // A tap only shows the tooltip: on a touch screen the links are in the table.
  container.addEventListener('click', () => {
    if (pointerType === 'mouse' && current !== null) marks?.open?.(current);
  });
  new ResizeObserver(() => draw(false)).observe(container);

  return {
    update(next) {
      data = next;
      draw(true);
    },
  };
}

/** The titles come from YouTube, so everything goes in as text, never as HTML. */
function fillTooltip(tooltip, { title, rows, note }) {
  const lines = [htmlNode('p', 'tooltip-title', title)];
  for (const row of rows) {
    const line = htmlNode('p', 'tooltip-row');
    if (row.key) line.append(htmlNode('span', `key ${row.key}`));
    line.append(htmlNode('strong', '', row.value), ' ', htmlNode('span', 'tooltip-label', row.label));
    lines.push(line);
  }
  if (note) lines.push(htmlNode('p', 'tooltip-note', note));
  tooltip.replaceChildren(...lines);
}

/** Puts the tooltip beside `anchor`, on the side where it fits, without leaving the chart. */
function placeTooltip(tooltip, anchor, container) {
  const roomLeft = Math.max(container.clientWidth - tooltip.offsetWidth, 0);
  let left = anchor.x + TOOLTIP_GAP;
  if (left > roomLeft) left = anchor.x - TOOLTIP_GAP - tooltip.offsetWidth;
  const top = anchor.y - tooltip.offsetHeight / 2;
  const roomTop = Math.max(container.clientHeight - tooltip.offsetHeight, 0);
  tooltip.style.transform = `translate(${clamp(left, 0, roomLeft)}px, ${clamp(top, 0, roomTop)}px)`;
}

/** The area of the chart the marks are drawn in, leaving room for the labels around it. */
function plotArea({ width, height }, yTicks, rightMargin) {
  const longest = Math.max(...yTicks.map((tick) => formatDurationShort(tick).length), 1);
  return { left: Math.round(longest * 6.6 + 16), right: width - rightMargin, top: 14, bottom: height - 30 };
}

function drawAxes(svg, { plot, y, yTicks, xTicks }) {
  const grid = svgNode('g', { class: 'grid' });
  const labels = svgNode('g', { class: 'tick-labels' });
  for (const tick of yTicks) {
    const position = Math.round(y(tick)) + 0.5;
    grid.append(svgNode('line', { x1: plot.left, x2: plot.right, y1: position, y2: position }));
    labels.append(
      svgNode('text', { x: plot.left - 8, y: position, dy: '0.32em', 'text-anchor': 'end' }, formatDurationShort(tick)),
    );
  }
  for (const tick of xTicks) {
    labels.append(svgNode('text', { x: tick.x, y: plot.bottom + 19, 'text-anchor': 'middle' }, tick.label));
  }
  const baseline = Math.round(plot.bottom) + 0.5;
  svg.append(grid, svgNode('line', { class: 'axis', x1: plot.left, x2: plot.right, y1: baseline, y2: baseline }), labels);
}

function timeAxis(times, x, plot) {
  const maxTicks = Math.max(2, Math.floor((plot.right - plot.left) / 80));
  return timeTicks(times[0], times.at(-1), maxTicks).map((tick) => ({ x: x(tick.time), label: tick.label }));
}

function linePath(videos, x, y) {
  return `M${videos.map((video) => `${x(video.time).toFixed(1)},${y(video.mean).toFixed(1)}`).join('L')}`;
}

/** The average duration over time, as a line. */
export function paintRate(svg, videos, size) {
  const times = videos.map((video) => video.time);
  const means = videos.map((video) => video.mean);
  const lowest = Math.min(...means);
  const highest = Math.max(...means);
  const padding = (highest - lowest || 1) * 0.08;
  const domain = [lowest - padding, highest + padding];
  const yTicks = durationTicks(domain[0], domain[1], size.narrow ? 5 : 6);
  const plot = plotArea(size, yTicks, size.narrow ? 16 : 104);
  const x = linear([times[0], times.at(-1)], [plot.left, plot.right]);
  const y = linear(domain, [plot.bottom, plot.top]);
  drawAxes(svg, { plot, y, yTicks, xTicks: timeAxis(times, x, plot) });

  const latest = videos.at(-1);
  svg.append(
    svgNode('path', { class: 'line s1', d: linePath(videos, x, y) }),
    svgNode('circle', { class: 'dot s1', cx: x(latest.time), cy: y(latest.mean), r: 4 }),
  );
  if (!size.narrow) {
    const label = { class: 'direct-label', x: x(latest.time) + 10, y: y(latest.mean), dy: '0.32em' };
    svg.append(svgNode('text', label, formatDuration(latest.mean)));
  }
  const crosshair = svgNode('line', { class: 'crosshair', y1: plot.top, y2: plot.bottom, visibility: 'hidden' });
  const marker = svgNode('circle', { class: 'dot s1 is-active', r: 5, visibility: 'hidden' });
  svg.append(crosshair, marker);

  return {
    count: videos.length,
    indexAt: (pointerX) =>
      pointerX < plot.left - 12 || pointerX > plot.right + 12 ? -1 : nearestIndex(times, x.invert(pointerX)),
    show(index) {
      const anchor = { x: x(times[index]), y: y(means[index]) };
      const line = Math.round(anchor.x) + 0.5;
      for (const [name, value] of [['x1', line], ['x2', line], ['visibility', 'visible']]) crosshair.setAttribute(name, value);
      for (const [name, value] of [['cx', anchor.x], ['cy', anchor.y], ['visibility', 'visible']]) marker.setAttribute(name, value);
      return anchor;
    },
    hide() {
      crosshair.setAttribute('visibility', 'hidden');
      marker.setAttribute('visibility', 'hidden');
    },
    describe(index) {
      const video = videos[index];
      const variation = `${formatVariation(video.variation)} (${formatPercent(video.variationPercent)})`;
      return {
        title: formatDate(video.time),
        rows: [
          { key: 'line s1', value: formatDuration(video.mean, { milliseconds: true }), label: 'Cotação' },
          { value: variation, label: 'com este vídeo' },
        ],
        note: `${video.title} · ${formatDuration(video.duration)}`,
      };
    },
  };
}

/** The duration of every video, as dots, with the average over time as a line. */
export function paintVideos(svg, videos, size) {
  const times = videos.map((video) => video.time);
  const domain = [0, Math.max(...videos.map((video) => video.duration)) * 1.05];
  const yTicks = durationTicks(0, domain[1], size.narrow ? 5 : 6);
  const plot = plotArea(size, yTicks, size.narrow ? 16 : 84);
  // Inset, so that the first and the last dots are not cut by the edges.
  const x = linear([times[0], times.at(-1)], [plot.left + 6, plot.right - 6]);
  const y = linear(domain, [plot.bottom, plot.top]);
  drawAxes(svg, { plot, y, yTicks, xTicks: timeAxis(times, x, plot) });

  const radius = size.narrow ? 2.5 : 3.5;
  const positions = videos.map((video) => ({ x: x(video.time), y: y(video.duration) }));
  const dots = svgNode('g', { class: 'dots' });
  for (const position of positions) {
    dots.append(svgNode('circle', { cx: position.x.toFixed(1), cy: position.y.toFixed(1), r: radius }));
  }
  const path = linePath(videos, x, y);
  svg.append(dots, svgNode('path', { class: 'line-casing', d: path }), svgNode('path', { class: 'line s1', d: path }));
  if (!size.narrow) {
    const latest = videos.at(-1);
    const label = { class: 'direct-label', x: x(latest.time) + 12, y: y(latest.mean), dy: '0.32em' };
    svg.append(svgNode('text', label, 'Cotação'));
  }
  const marker = svgNode('circle', { class: 'dot s2 is-active', r: radius + 2.5, visibility: 'hidden' });
  svg.append(marker);

  return {
    count: videos.length,
    indexAt(pointerX, pointerY) {
      let nearest = -1;
      let nearestDistance = HOVER_RADIUS ** 2;
      positions.forEach((position, index) => {
        const distance = (position.x - pointerX) ** 2 + (position.y - pointerY) ** 2;
        if (distance < nearestDistance) [nearest, nearestDistance] = [index, distance];
      });
      return nearest;
    },
    show(index) {
      const anchor = positions[index];
      for (const [name, value] of [['cx', anchor.x], ['cy', anchor.y], ['visibility', 'visible']]) marker.setAttribute(name, value);
      svg.style.cursor = 'pointer';
      return anchor;
    },
    hide() {
      marker.setAttribute('visibility', 'hidden');
      svg.style.cursor = '';
    },
    describe(index) {
      const video = videos[index];
      return {
        title: video.title,
        rows: [
          { key: 'dot s2', value: formatDuration(video.duration), label: 'Duração' },
          { key: 'line s1', value: formatDuration(video.mean), label: 'Cotação na data' },
        ],
        note: [formatDate(video.time), formatKind(video.kind)].filter(Boolean).join(' · '),
      };
    },
    open(index) {
      window.open(videos[index].url, '_blank', 'noopener');
    },
  };
}

/** The average duration of the videos of each year or month, as columns. */
export function paintPeriods(svg, groups, size) {
  const highest = Math.max(...groups.map((group) => group.mean ?? 0)) || 1;
  const domain = [0, highest * 1.12];
  const yTicks = durationTicks(0, domain[1], size.narrow ? 4 : 5);
  const plot = plotArea(size, yTicks, 12);
  const y = linear(domain, [plot.bottom, plot.top]);
  const band = (plot.right - plot.left) / groups.length;
  // Never wider than 24px, and always 2px apart from the neighbours.
  const barWidth = clamp(band - 2, 1, 24);
  const center = (index) => plot.left + band * (index + 0.5);

  // Only as many labels as fit, counted back from the most recent period.
  const everyNth = Math.max(1, Math.ceil((groups[0].label.length * 7 + 12) / band));
  const xTicks = groups
    .map((group, index) => ({ x: center(index), label: group.label }))
    .filter((_, index) => (groups.length - 1 - index) % everyNth === 0);
  drawAxes(svg, { plot, y, yTicks, xTicks });

  const bars = groups.map((group, index) => {
    if (group.mean === null) return null;
    const bar = svgNode('path', { class: 'bar', d: columnPath(center(index) - barWidth / 2, y(group.mean), barWidth, plot.bottom) });
    svg.append(bar);
    return bar;
  });
  for (const index of labelledColumns(groups)) {
    const text = columnLabel(groups[index].mean);
    const halfWidth = text.length * 3.4;
    const position = clamp(center(index), plot.left + halfWidth, size.width - halfWidth - 2);
    svg.append(svgNode('text', { class: 'direct-label', x: position, y: y(groups[index].mean) - 7, 'text-anchor': 'middle' }, text));
  }

  return {
    count: groups.length,
    indexAt: (pointerX) =>
      pointerX < plot.left || pointerX >= plot.right ? -1 : Math.min(Math.floor((pointerX - plot.left) / band), groups.length - 1),
    show(index) {
      for (const bar of bars) bar?.classList.remove('is-active');
      bars[index]?.classList.add('is-active');
      const mean = groups[index].mean;
      return { x: center(index) + barWidth / 2, y: mean === null ? (plot.top + plot.bottom) / 2 : y(mean) };
    },
    hide() {
      for (const bar of bars) bar?.classList.remove('is-active');
    },
    describe(index) {
      const group = groups[index];
      if (!group.count) return { title: group.label, rows: [{ value: 'Nenhum', label: 'vídeo publicado' }] };
      return {
        title: group.label,
        rows: [
          { key: 'box s1', value: formatDuration(group.mean), label: 'Duração média' },
          { value: formatCount(group.count), label: group.count === 1 ? 'vídeo' : 'vídeos' },
          { value: formatTotal(group.total), label: 'no total' },
        ],
      };
    },
  };
}

/** A column with its top corners rounded and its base square. */
function columnPath(left, top, width, bottom) {
  const radius = Math.max(Math.min(4, width / 2, bottom - top), 0);
  const right = left + width;
  return `M${left},${bottom}V${top + radius}Q${left},${top} ${left + radius},${top}H${right - radius}Q${right},${top} ${right},${top + radius}V${bottom}Z`;
}

/** The columns worth a label: the highest, and the most recent when the column before it leaves room. */
function labelledColumns(groups) {
  const latest = groups.length - 1;
  let highest = -1;
  groups.forEach((group, index) => {
    if (group.mean !== null && (highest < 0 || group.mean > groups[highest].mean)) highest = index;
  });
  const labelled = new Set(highest < 0 ? [] : [highest]);
  const before = latest > 0 ? (groups[latest - 1].mean ?? 0) : 0;
  if (groups[latest].mean !== null && before <= groups[latest].mean) labelled.add(latest);
  return labelled;
}

/** Whole minutes for long averages, so that the label stays short. */
function columnLabel(mean) {
  return formatDurationShort(mean >= 600 ? Math.round(mean / 60) * 60 : Math.round(mean));
}
