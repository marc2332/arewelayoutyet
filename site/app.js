const labels = { yes: '● Supported', partial: '◐ Partial', different: '≈ Different model', no: '○ Not supported', unknown: '? Unverified' };
const element = (tag, text, className) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
};
const link = (text, url) => {
  const node = element('a', text);
  node.href = url;
  return node;
};
function badge(description, url) {
  const image = element('img', undefined, 'metric-badge');
  image.alt = description;
  image.src = url;
  image.height = 20;
  image.loading = 'lazy';
  image.decoding = 'async';
  image.addEventListener('error', () => image.replaceWith(element('span', '/')));
  return image;
}
async function loadJson(path) {
  const response = await fetch(path, { cache: 'no-cache' });
  if (!response.ok) throw new Error(`Could not load ${path} (${response.status})`);
  return response.json();
}
function renderFeatures(data) {
  const table = document.querySelector('#features');
  table.replaceChildren(element('caption', 'Capabilities of the pinned builds. Cells link to supporting APIs or source.'));
  const head = element('thead');
  const headings = element('tr');
  const title = element('th', 'Library');
  title.scope = 'col';
  headings.append(title);
  for (const library of data.libraries) {
    const heading = element('th');
    heading.scope = 'col';
    heading.append(link(library.name, library.url), element('small', library.version));
    headings.append(heading);
  }
  head.append(headings);
  const body = element('tbody');
  const releaseRow = element('tr', undefined, 'release-date');
  const releaseHeading = element('th', 'Release date');
  releaseHeading.scope = 'row';
  releaseRow.append(releaseHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    const date = element('time', library.releaseDate);
    date.dateTime = library.releaseDate;
    date.title = 'Publication date on crates.io for the listed version. Git-pinned builds may include newer changes.';
    cell.append(date);
    releaseRow.append(cell);
  }
  body.append(releaseRow);
  const downloadsRow = element('tr', undefined, 'downloads');
  const downloadsHeading = element('th', 'crates.io downloads');
  downloadsHeading.scope = 'row';
  downloadsRow.append(downloadsHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    const crateLink = link(undefined, `${library.url}/downloads`);
    crateLink.title = `All-time downloads of the ${library.id} Rust crate across all versions`;
    crateLink.append(badge(`Total crates.io downloads for ${library.id}`,
      `https://img.shields.io/crates/d/${encodeURIComponent(library.id)}?style=flat&color=555&labelColor=333&label=total`));
    cell.append(crateLink);
    downloadsRow.append(cell);
  }
  body.append(downloadsRow);
  const usedByRow = element('tr', undefined, 'used-by');
  const usedByHeading = element('th', 'Used by');
  usedByHeading.scope = 'row';
  usedByRow.append(usedByHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    library.usedBy.forEach((project, index) => {
      if (index > 0) cell.append(', ');
      const projectLink = link(project.name, project.url);
      if (project.note) projectLink.title = project.note;
      cell.append(projectLink);
      if (project.note) cell.append(element('small', project.note));
    });
    if (!library.usedBy.length) cell.append('/');
    usedByRow.append(cell);
  }
  body.append(usedByRow);
  for (const feature of data.features) {
    const row = element('tr');
    const heading = element('th', feature.name);
    heading.scope = 'row';
    row.append(heading);
    feature.support.forEach((status, index) => {
      const cell = element('td', undefined, status);
      const library = data.libraries[index];
      const source = link(labels[status], feature.sources?.[library.id] ?? library.source);
      source.title = feature.note;
      source.setAttribute('aria-label', `${feature.name}: ${labels[status]}. ${feature.note}`);
      cell.append(source);
      row.append(cell);
    });
    body.append(row);
  }
  table.append(head, body);
}
function formatTime(nanoseconds) {
  if (nanoseconds >= 1e6) return `${(nanoseconds / 1e6).toFixed(2)} ms`;
  if (nanoseconds >= 1e3) return `${(nanoseconds / 1e3).toFixed(2)} µs`;
  return `${nanoseconds.toFixed(1)} ns`;
}
function renderBenchmarkChart(run, libraries, scenarios, hiddenLibraries) {
  const mode = document.querySelector('#benchmark-chart-mode').value;
  const relativeMode = mode === 'relative';
  const milliseconds = mode === 'milliseconds';
  const perNode = mode === 'per-node';
  const formatAbsolute = value => `${Number(value.toPrecision(4))} ${perNode ? 'ns/node' : 'ms'}`;
  const formatChartTime = nanoseconds => milliseconds ? formatAbsolute(nanoseconds / 1e6) : formatTime(nanoseconds);
  const hasNodeCount = scenario => Number.isSafeInteger(run.node_counts?.[scenario]) && run.node_counts[scenario] > 0;
  const captions = {
    relative: 'Further left is faster. The fastest visible mean per benchmark is 1×. Dots and 95% confidence bounds are divided by that fastest visible mean. The X axis is logarithmic. Missing results have no dot.',
    milliseconds: 'Further left is faster. Dots show mean time in milliseconds, with 95% confidence-interval whiskers. The X axis is logarithmic. Missing results have no dot.',
    'per-node': 'Further left is faster. Mean times and 95% confidence bounds are divided by the workload node count, including the root, and shown in ns/node. The X axis is logarithmic. This is average cost, not an individual node timing. Missing results have no dot.',
  };
  const missingCounts = perNode && scenarios.some(scenario => !hasNodeCount(scenario));
  document.querySelector('#benchmark-chart-caption').textContent = captions[mode] + (missingCounts ? ' Workloads without recorded node counts are omitted.' : '');
  const namespace = 'http://www.w3.org/2000/svg';
  const svgElement = (tag, attributes = {}, text) => {
    const node = document.createElementNS(namespace, tag);
    for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const left = 240;
  const top = 62;
  const width = 1120;
  const right = width - 32;
  const laneHeight = 16;
  const colors = { taffy: '#60a5fa', morphorm: '#c084fc', torin: '#34d399', yoga: '#fb923c', ui_layout: '#fb7185', 'clay-layout': '#facc15', 'ply-engine': '#22d3ee', backer: '#e2e8f0', keru_layout: '#a3e635' };
  const visibleLibraries = libraries.filter(library => !hiddenLibraries.has(library.id));
  const measurements = scenarios.map(scenario => {
    const nodeCount = run.node_counts?.[scenario];
    const results = run.results.filter(result => result.scenario === scenario && visibleLibraries.some(library => library.id === result.library) && (!perNode || hasNodeCount(scenario)));
    const fastest = Math.min(...results.map(result => result.mean_ns));
    const divisor = perNode ? nodeCount : milliseconds ? 1e6 : fastest;
    return { results, fastest, nodeCount, divisor };
  });
  const values = measurements.flatMap(({ results, divisor }) => results.flatMap(result => [result.lower_ns / divisor, result.upper_ns / divisor]));
  const minimum = values.length ? Math.min(...values) : 1;
  const firstTick = relativeMode ? 1 : 10 ** Math.floor(Math.log10(minimum));
  const axisMinimum = relativeMode ? Math.min(1, minimum) * 0.98 : firstTick;
  const maximum = Math.max(firstTick * 2, ...values);
  const ticks = [firstTick];
  for (let magnitude = firstTick; ticks.at(-1) < maximum; magnitude *= 10) {
    for (const factor of [2, 5, 10]) {
      ticks.push(magnitude * factor);
      if (ticks.at(-1) >= maximum) break;
    }
  }
  const positionX = value => left + Math.log(value / axisMinimum) / Math.log(ticks.at(-1) / axisMinimum) * (right - left);
  let bottom = top;
  const rows = measurements.map(({ results, divisor }) => {
    const points = visibleLibraries.flatMap(library => {
      const result = results.find(result => result.library === library.id);
      if (!result) return [];
      const horizontal = positionX(result.mean_ns / divisor);
      const lower = positionX(result.lower_ns / divisor);
      const upper = positionX(result.upper_ns / divisor);
      const labelWidth = library.name.length * 7;
      const labelOnLeft = horizontal + 12 + labelWidth > width - 16;
      const start = Math.min(lower, horizontal - 6, labelOnLeft ? horizontal - 15 - labelWidth : horizontal);
      const end = Math.max(upper, horizontal + 6, labelOnLeft ? horizontal : horizontal + 15 + labelWidth);
      return [{ id: library.id, result, horizontal, lower, upper, labelOnLeft, start, end }];
    }).sort((first, second) => first.start - second.start);
    const laneEnds = [];
    for (const point of points) {
      let lane = laneEnds.findIndex(end => end + 12 <= point.start);
      if (lane === -1) lane = laneEnds.length;
      laneEnds[lane] = point.end;
      point.lane = lane;
    }
    const rowHeight = Math.max(32, laneEnds.length * laneHeight + 12);
    for (const point of points) {
      point.vertical = bottom + rowHeight / 2 + (point.lane - (laneEnds.length - 1) / 2) * laneHeight;
    }
    const row = { vertical: bottom, height: rowHeight, points: new Map(points.map(point => [point.id, point])) };
    bottom += rowHeight;
    return row;
  });
  const height = bottom + 56;
  const axisLabel = perNode ? 'Average time per node (ns/node, log scale), faster to the left' : milliseconds ? 'Mean time in milliseconds (log scale), faster to the left' : 'Relative mean time (log scale), faster to the left';
  const svg = svgElement('svg', { width, height, viewBox: `0 0 ${width} ${height}`, role: 'img', 'aria-labelledby': 'chart-title chart-description' });
  svg.style.setProperty('--chart-width', `${width}px`);
  svg.append(
    svgElement('title', { id: 'chart-title' }, perNode ? 'Per-node layout benchmark dot plots' : milliseconds ? 'Layout benchmark dot plots in milliseconds' : 'Relative layout benchmark dot plots'),
    svgElement('desc', { id: 'chart-description' }, `Each benchmark has its own row. Labeled colored dots show library means, sharing lanes when their labels and confidence intervals do not overlap. ${captions[mode]} Exact total times and confidence intervals are also available in the results table.`),
    svgElement('text', { x: left, y: 22, fill: '#a1a1aa', 'font-size': 11 }, axisLabel),
    svgElement('text', { x: left - 16, y: 22, 'text-anchor': 'end', fill: '#a1a1aa', 'font-size': 11 }, 'Benchmarks'),
    svgElement('text', { x: (left + right) / 2, y: height - 12, 'text-anchor': 'middle', fill: '#a1a1aa', 'font-size': 11 }, axisLabel),
  );
  scenarios.forEach((scenario, index) => {
    const { vertical, height: rowHeight } = rows[index];
    svg.append(
      svgElement('rect', { x: 8, y: vertical, width: width - 16, height: rowHeight, fill: index % 2 === 0 ? '#1c1c1c' : '#181818' }),
      svgElement('line', { x1: 8, x2: width - 8, y1: vertical + rowHeight, y2: vertical + rowHeight, stroke: '#292929' }),
      svgElement('text', { x: left - 16, y: vertical + rowHeight / 2 + 4, 'text-anchor': 'end', fill: '#e4e4e7', 'font-size': 12 }, scenario.replaceAll('_', ' ')),
    );
  });
  for (const tick of ticks) {
    const horizontal = positionX(tick);
    const baseline = relativeMode && tick === 1;
    const label = relativeMode ? tick === 1 ? '1× fastest' : `${tick}×` : formatAbsolute(tick);
    svg.append(
      svgElement('line', { x1: horizontal, x2: horizontal, y1: top, y2: bottom, stroke: baseline ? '#71717a' : '#303030' }),
      svgElement('text', { x: horizontal, y: top - 12, 'text-anchor': 'middle', fill: baseline ? '#e4e4e7' : '#a1a1aa', 'font-size': 11 }, label),
      svgElement('text', { x: horizontal, y: bottom + 20, 'text-anchor': 'middle', fill: baseline ? '#e4e4e7' : '#a1a1aa', 'font-size': 11 }, label),
    );
  }
  const legend = document.querySelector('#benchmark-chart-legend');
  legend.replaceChildren();
  const seriesGroups = [];
  const highlight = selected => {
    for (const { id, group } of seriesGroups) group.style.opacity = selected === null || id === selected ? '1' : '0.15';
  };
  libraries.forEach((library, libraryIndex) => {
    const color = colors[library.id] ?? '#d4d4d8';
    const group = svgElement('g', { class: 'chart-series' });
    group.style.display = hiddenLibraries.has(library.id) ? 'none' : '';
    seriesGroups.push({ id: library.id, group });
    const dots = [];
    measurements.forEach(({ fastest, nodeCount }, index) => {
      const layout = rows[index].points.get(library.id);
      if (!layout) return;
      const { result, horizontal, lower, upper, vertical, labelOnLeft } = layout;
      const point = svgElement('g');
      const relative = result.mean_ns / fastest;
      const formatMeasuredTime = nanoseconds => perNode ? formatAbsolute(nanoseconds / nodeCount) : formatChartTime(nanoseconds);
      const mean = relativeMode ? `${relative.toFixed(2)}× the fastest visible mean, ${formatMeasuredTime(result.mean_ns)}` : `${formatMeasuredTime(result.mean_ns)} mean`;
      const normalization = perNode ? ` Across ${nodeCount.toLocaleString('en-US')} workload nodes including the root, ${formatTime(result.mean_ns)} total.` : '';
      point.append(
        svgElement('title', {}, `${library.name}, ${scenarios[index].replaceAll('_', ' ')}: ${mean}. 95% confidence interval ${formatMeasuredTime(result.lower_ns)} to ${formatMeasuredTime(result.upper_ns)}.${normalization}`),
        svgElement('line', { x1: lower, x2: upper, y1: vertical, y2: vertical, stroke: color, 'stroke-width': 1.5 }),
        svgElement('line', { x1: lower, x2: lower, y1: vertical - 2.5, y2: vertical + 2.5, stroke: color, 'stroke-width': 1.5 }),
        svgElement('line', { x1: upper, x2: upper, y1: vertical - 2.5, y2: vertical + 2.5, stroke: color, 'stroke-width': 1.5 }),
        svgElement('circle', { cx: horizontal, cy: vertical, r: 5, fill: color, stroke: '#181818', 'stroke-width': 1 }),
        svgElement('text', {
          x: horizontal + (labelOnLeft ? -12 : 12), y: vertical + 4,
          'text-anchor': labelOnLeft ? 'end' : 'start', fill: color, 'font-size': 11,
          stroke: '#181818', 'stroke-width': 3, 'stroke-linejoin': 'round', 'paint-order': 'stroke',
        }, library.name),
      );
      dots.push(point);
    });
    group.append(...dots);
    svg.append(group);
    const button = element('button');
    button.type = 'button';
    const hasMeasurements = run.results.some(result => result.library === library.id && (!perNode || hasNodeCount(result.scenario)));
    button.disabled = !hasMeasurements;
    button.setAttribute('aria-pressed', String(hasMeasurements && !hiddenLibraries.has(library.id)));
    button.setAttribute('aria-controls', 'benchmark-chart');
    button.title = hasMeasurements ? `Toggle ${library.name}. Hover or focus to highlight.` : `${library.name}: no measurements in this run`;
    const swatch = element('span', undefined, 'chart-swatch');
    swatch.style.color = color;
    swatch.setAttribute('aria-hidden', 'true');
    button.append(swatch, library.name);
    const highlightCurrent = () => highlight(button.getAttribute('aria-pressed') === 'true' ? library.id : null);
    button.addEventListener('click', () => {
      const visible = button.getAttribute('aria-pressed') !== 'true';
      if (visible) hiddenLibraries.delete(library.id);
      else hiddenLibraries.add(library.id);
      const restoreFocus = document.activeElement === button;
      renderBenchmarkChart(run, libraries, scenarios, hiddenLibraries);
      if (restoreFocus) legend.children[libraryIndex].focus({ preventScroll: true });
    });
    button.addEventListener('pointerenter', highlightCurrent);
    button.addEventListener('pointerleave', () => highlight(null));
    button.addEventListener('focus', highlightCurrent);
    button.addEventListener('blur', () => highlight(null));
    legend.append(button);
  });
  const chart = document.querySelector('#benchmark-chart');
  if (values.length) {
    chart.replaceChildren(svg);
  } else {
    const message = perNode && !scenarios.some(hasNodeCount)
      ? 'Node counts are unavailable for this run. Choose another view or a newer run.'
      : 'No visible measurements. Enable a library above to show its results.';
    const empty = element('p', message, 'chart-empty small');
    empty.setAttribute('role', 'status');
    chart.replaceChildren(empty);
  }
}
function renderBenchmarks(history, libraries) {
  if (history.schema !== 1 || !Array.isArray(history.runs)) throw new Error('Unsupported history format');
  const runs = [...history.runs].sort((first, second) => second.date.localeCompare(first.date));
  const status = document.querySelector('#benchmark-status');
  if (!runs.length) {
    status.textContent = 'No measurements published yet.';
    return;
  }
  const select = document.querySelector('#benchmark-run');
  select.replaceChildren();
  runs.forEach((run, index) => {
    const option = element('option', `${new Date(run.date).toISOString().replace('T', ' ').slice(0, 19)} UTC${index === 0 ? ' (latest)' : ''}`);
    option.value = String(index);
    select.append(option);
  });
  select.value = '0';
  const hiddenLibraries = new Set();
  document.querySelector('#benchmark-chart-mode').addEventListener('change', () => {
    const run = runs[Number(select.value)];
    const scenarios = [...new Set(run.results.map(result => result.scenario))].sort();
    renderBenchmarkChart(run, libraries, scenarios, hiddenLibraries);
  });
  const render = () => {
    const run = runs[Number(select.value)];
    const table = document.querySelector('#benchmark-results');
    const frameLibraries = libraries.filter(library => ['clay-layout', 'ply-engine'].includes(library.id) && run.versions[library.id]).map(library => library.name);
    const timingNote = frameLibraries.length ? ` ${frameLibraries.join(' and ')} timings include native element declaration and render-command generation.` : '';
    table.replaceChildren(element('caption', `Mean layout time, lower is faster. Ranges are 95% confidence intervals. Uncached layout, adapters can differ in semantics.${timingNote}`));
    const head = element('thead');
    const headings = element('tr');
    const workloadHeading = element('th', 'Workload');
    workloadHeading.scope = 'col';
    headings.append(workloadHeading);
    for (const library of libraries) {
      const heading = element('th');
      heading.scope = 'col';
      heading.append(link(library.name, library.url), element('small', run.versions[library.id] ?? 'Not measured'));
      headings.append(heading);
    }
    head.append(headings);
    const body = element('tbody');
    const scenarios = [...new Set(run.results.map(result => result.scenario))].sort();
    renderBenchmarkChart(run, libraries, scenarios, hiddenLibraries);
    for (const scenario of scenarios) {
      const results = run.results.filter(result => result.scenario === scenario && libraries.some(library => library.id === result.library));
      const fastestMean = Math.min(...results.map(result => result.mean_ns));
      const slowestMean = Math.max(...results.map(result => result.mean_ns));
      const row = element('tr');
      const heading = element('th', scenario.replaceAll('_', ' '));
      heading.scope = 'row';
      row.append(heading);
      for (const library of libraries) {
        const result = results.find(result => result.library === library.id);
        const cell = element('td');
        if (result) {
          const mean = element('span', formatTime(result.mean_ns));
          if (result.mean_ns === fastestMean) {
            mean.className = 'yes';
            mean.title = 'Lowest mean time in this row';
          } else if (result.mean_ns === slowestMean) {
            mean.className = 'worst';
            mean.title = 'Highest mean time in this row';
          }
          cell.append(mean, element('small', `${formatTime(result.lower_ns)} to ${formatTime(result.upper_ns)}`));
        } else {
          const missing = element('span', '/', 'small');
          missing.title = run.versions[library.id] ? 'Not supported by adapter' : 'Not benchmarked in this run';
          cell.append(missing);
        }
        row.append(cell);
      }
      body.append(row);
    }
    table.append(head, body);
    const details = document.querySelector('#run-details');
    details.replaceChildren(`${run.cpu} · ${run.rustc} · ${run.runner}`);
    if (run.url) details.append(' · ', link('CI run', run.url));
    status.textContent = `${runs.length} recorded ${runs.length === 1 ? 'run' : 'runs'}. Missing results are not zero time.`;
  };
  select.addEventListener('change', render);
  document.querySelector('#benchmark-content').hidden = false;
  render();
}
async function main() {
  let features;
  try {
    features = await loadJson('data/features.json');
    renderFeatures(features);
  } catch (error) {
    document.querySelector('#features caption').textContent = `Feature data unavailable. ${error.message}`;
  }
  try {
    const response = await fetch('data/history.json', { cache: 'no-cache' });
    if (response.status === 404) {
      document.querySelector('#benchmark-status').textContent = 'No measurements published yet. Run just fetch-data for local builds, or check the publishing workflow.';
      return;
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (!features) throw new Error('Library definitions unavailable');
    renderBenchmarks(await response.json(), features.libraries);
  } catch (error) {
    document.querySelector('#benchmark-content').hidden = true;
    document.querySelector('#benchmark-status').textContent = `Measurements unavailable. ${error.message}`;
  }
}
main();
