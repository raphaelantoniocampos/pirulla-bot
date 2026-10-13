// What the page computes from the videos in the data file.

const TIME_ZONE = 'America/Sao_Paulo';
const YEAR_MS = 365.25 * 86_400_000;
const MONTHS = ['jan', 'fev', 'mar', 'abr', 'mai', 'jun', 'jul', 'ago', 'set', 'out', 'nov', 'dez'];

const yearAndMonth = new Intl.DateTimeFormat('en-CA', { timeZone: TIME_ZONE, year: 'numeric', month: '2-digit' });

/** Turns the videos of the data file into what the page plots. They must be in order of publication. */
export function prepare(videos) {
  return videos.map((video, index) => {
    const previousMean = index ? videos[index - 1].mean : video.mean;
    const variation = video.mean - previousMean;
    return {
      id: video.id,
      title: video.title,
      time: Date.parse(video.publishedAt),
      duration: video.duration,
      mean: video.mean,
      variation,
      variationPercent: previousMean ? (variation / previousMean) * 100 : 0,
      url: `https://www.youtube.com/watch?v=${video.id}`,
    };
  });
}

/** The videos published in the `years` before `now`; all of them when `years` is null. */
export function lastYears(videos, years, now) {
  if (years === null) return videos;
  const since = now - years * YEAR_MS;
  return videos.filter((video) => video.time >= since);
}

/** Totals for each year or month, in the time of São Paulo, including the periods without videos. */
export function groupByPeriod(videos, period) {
  if (!videos.length) return [];
  const first = periodIndex(videos[0].time, period);
  const last = periodIndex(videos.at(-1).time, period);
  const groups = Array.from({ length: last - first + 1 }, (_, offset) => emptyGroup(first + offset, period));

  for (const video of videos) {
    const group = groups[periodIndex(video.time, period) - first];
    group.count += 1;
    group.total += video.duration;
  }
  for (const group of groups) group.mean = group.count ? group.total / group.count : null;
  return groups;
}

/** The year, or the number of months since year zero, that `time` falls in. */
function periodIndex(time, period) {
  const parts = yearAndMonth.formatToParts(time);
  const year = Number(parts.find((part) => part.type === 'year').value);
  const month = Number(parts.find((part) => part.type === 'month').value);
  return period === 'year' ? year : year * 12 + month - 1;
}

function emptyGroup(index, period) {
  const totals = { count: 0, total: 0, mean: null };
  if (period === 'year') return { key: String(index), label: String(index), ...totals };
  const year = Math.floor(index / 12);
  const month = index % 12;
  return {
    key: `${year}-${String(month + 1).padStart(2, '0')}`,
    label: `${MONTHS[month]}/${String(year).slice(-2)}`,
    ...totals,
  };
}

/** How many videos there are, how long they are together and which is the longest. */
export function summarize(videos) {
  let total = 0;
  let longest = null;
  for (const video of videos) {
    total += video.duration;
    if (!longest || video.duration > longest.duration) longest = video;
  }
  return { count: videos.length, total, longest };
}
