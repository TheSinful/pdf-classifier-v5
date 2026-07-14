/**
 * app.js — UI wiring for the classifier run visualizer.
 * Consumes parser.js (LogParser) + model.js (LogModel).
 */
(function () {
  "use strict";

  const $ = (id) => document.getElementById(id);
  const el = (tag, cls, text) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  };
  const esc = (s) => String(s ?? "");

  // ---------- global state ----------
  const S = {
    model: null,
    cur: -1,             // current action seq (-1 = before anything)
    visible: [],         // seqs of actions shown in the stepper (minor-filtered)
    stateCache: { idx: null, state: null },
    classColors: {},     // CLASS -> css var
    classOrder: [],
    playTimer: null,
    tab: "step",
    cells: [],           // strip cell elements by page offset
    cellPages: [],
    logWindow: { start: -1, end: -1 },
    stepsWindow: { g0: -1, g1: -1 },
    live: null,          // {id, es, stderr:[], stdout:[], extracts:[], status}
    serverOk: false,
    reparseTimer: null,
    accuracy: null,      // lazily computed ground-truth comparison (per model)
    treeCollapsed: new Set(),  // container pages collapsed by the user
    treeFailsOpen: new Set(),  // anchor pages with failure branch expanded
  };

  const SLOT_VARS = ["--s1", "--s2", "--s3", "--s4", "--s5", "--s6", "--s7", "--s8"];

  const TYPE_ICONS = {
    infer: "☰", decide: "◆", classify_result: "✓", extract_result: "⇣",
    job_push: "+", poll: "↻", worker_assign: "·", advance: "→",
    override_eval: "§", handle_override: "§", defer_signal: "⚠",
    schedule_deferral: "⚠", deferral_begin: "🔍", probe: "?",
    eliminate: "✗", anchor_found: "⚓", fill_begin: "▤", fill_strategy: "▤",
    fill_range: "▤", schedule_ovstr: "≋", ovstr_begin: "≋", ovstr_step: "≋",
    ovstr_exit_check: "?", ovstr_exit: "≋", ovstr_container: "≋",
    error: "✗", warn: "▲", parent_update: "⌂", boot: "●", note: "·",
    step_begin: "·", span: "·",
  };

  // ---------- loading ----------

  function loadText(text, opts = {}) {
    const parsed = LogParser.parseLog(text);
    S.model = LogModel.buildModel(parsed, { extracts: opts.extracts || [] });
    S.stateCache = { idx: null, state: null };
    S.accuracy = null;
    assignClassColors();
    rebuildVisible();

    document.body.classList.add("loaded");
    $("app").classList.remove("empty");
    $("empty-hint").hidden = true;
    $("strip-section").hidden = false;
    $("panels").hidden = false;
    $("playbar").hidden = false;

    buildStrip();
    buildLegend();
    S.stepsWindow = { g0: -1, g1: -1 };
    S.logWindow = { start: -1, end: -1 };

    const target = opts.keepPosition && S.cur >= 0
      ? Math.min(S.cur, S.model.actions.length - 1)
      : (opts.goToEnd ? lastVisible() : firstVisible());
    seek(target, { force: true });
  }

  function assignClassColors() {
    const order = [];
    const seen = new Set();
    const firstInfer = S.model.actions.find((a) => a.type === "infer");
    if (firstInfer) for (const c of firstInfer.infer.candidates) { if (!seen.has(c)) { seen.add(c); order.push(c); } }
    for (const a of S.model.actions) {
      if (a.class && !seen.has(a.class)) { seen.add(a.class); order.push(a.class); }
    }
    for (const r of S.model.results) if (!seen.has(r.class)) { seen.add(r.class); order.push(r.class); }
    S.classColors = {};
    S.classOrder = order;
    let slot = 0;
    for (const c of order) {
      if (c === "UNKNOWN") { S.classColors[c] = "var(--baseline)"; continue; }
      S.classColors[c] = slot < SLOT_VARS.length ? `var(${SLOT_VARS[slot]})` : "var(--muted)";
      slot++;
    }
  }

  const colorOf = (cls) => S.classColors[cls] || "var(--muted)";

  function rebuildVisible() {
    const showMinor = $("show-minor").checked;
    S.visible = [];
    for (const a of S.model.actions) if (showMinor || !a.minor) S.visible.push(a.seq);
    if (!S.visible.length) S.visible = S.model.actions.map((a) => a.seq);
    $("pb-scrub").max = String(S.visible.length - 1);
  }

  const firstVisible = () => S.visible[0] ?? 0;
  const lastVisible = () => S.visible[S.visible.length - 1] ?? 0;

  function visPos(seq) {
    // nearest visible position <= seq
    let lo = 0, hi = S.visible.length - 1, ans = 0;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (S.visible[mid] <= seq) { ans = mid; lo = mid + 1; } else hi = mid - 1;
    }
    return ans;
  }

  // ---------- page strip ----------

  function buildStrip() {
    const strip = $("strip");
    const ticks = $("strip-ticks");
    strip.textContent = "";
    ticks.textContent = "";
    S.cells = [];
    S.cellPages = [];
    const { fromPage, toPage } = S.model.meta;
    if (fromPage == null || toPage == null) return;
    const w = +$("zoom").value;
    const tickEvery = w >= 22 ? 5 : w >= 12 ? 10 : w >= 8 ? 20 : 50;

    for (let p = fromPage; p < toPage; p++) {
      const c = el("div", "cell");
      c.style.width = w + "px";
      c.dataset.page = p;
      strip.appendChild(c);
      S.cells.push(c);
      S.cellPages.push(p);

      const t = el("span", null, p % tickEvery === 0 ? String(p) : "");
      t.style.width = w + 2 + "px";
      ticks.appendChild(t);
    }
    $("strip-range").textContent = `pages ${fromPage}–${toPage - 1}`;
  }

  function updateStrip(st, act) {
    const { fromPage } = S.model.meta;
    for (let i = 0; i < S.cells.length; i++) {
      const p = S.cellPages[i];
      const c = S.cells[i];
      const cls = st.pages[p];
      c.className = "cell" + (cls ? " assigned" : "");
      c.style.background = cls ? colorOf(cls) : "transparent";
      c.textContent = "";
      const v = st.verdicts[p];
      if (v) {
        const m = el("div", "vmark " + (v.status === "verified" ? "verified" : "mismatch"));
        c.appendChild(m);
        if (v.status === "mismatch") c.appendChild(el("div", "xmark", "✗"));
      }
      const ex = st.extractions[p];
      if (ex && !ex.ok) c.appendChild(el("div", "emark"));
      if (st.probes[p]) c.classList.add("probe");
      if (st.cursor === p) c.classList.add("cursor");
    }
    // deferral bands
    const bands = $("strip-bands");
    bands.textContent = "";
    const w = +$("zoom").value + 2;
    const mk = (from, to, active) => {
      const b = el("div", "band" + (active ? " active" : ""));
      b.style.left = (from - fromPage) * w + "px";
      b.style.width = Math.max(w - 2, (to - from + 1) * w - 2) + "px";
      bands.appendChild(b);
    };
    for (const r of st.deferRegions) mk(r.from, r.to, false);
    if (st.mode === "DEFERRAL" && st.deferFrom != null) mk(st.deferFrom, st.cursor ?? st.deferFrom, true);

    // keep cursor in view
    const cur = S.cells[(st.cursor ?? fromPage) - fromPage];
    if (cur) {
      const sc = $("strip-scroll");
      const x = cur.offsetLeft;
      if (x < sc.scrollLeft + 40 || x > sc.scrollLeft + sc.clientWidth - 60) {
        sc.scrollLeft = x - sc.clientWidth / 2;
      }
    }
  }

  function buildLegend() {
    const lg = $("legend");
    lg.textContent = "";
    for (const c of S.classOrder) {
      const k = el("span", "key");
      const sw = el("span", "swatch");
      sw.style.background = colorOf(c);
      if (c === "UNKNOWN") sw.style.border = "1px solid var(--muted)";
      k.appendChild(sw);
      k.appendChild(el("span", null, c.toLowerCase()));
      lg.appendChild(k);
    }
    const keys = [
      ["✓", "verified", "var(--good)"],
      ["✗", "mismatch", "var(--critical)"],
      ["▲", "extract fail", "var(--serious)"],
      ["▨", "deferred region", "var(--warning)"],
    ];
    for (const [g, label, col] of keys) {
      const k = el("span", "key");
      const gl = el("span", "glyph", g);
      gl.style.color = col;
      k.appendChild(gl);
      k.appendChild(el("span", null, label));
      lg.appendChild(k);
    }
  }

  // strip tooltip + click
  (function stripEvents() {
    const tip = $("cell-tip");
    $("strip").addEventListener("mousemove", (e) => {
      const cell = e.target.closest(".cell");
      if (!cell) { tip.hidden = true; return; }
      const p = +cell.dataset.page;
      const st = curState();
      const lines = [`page ${p}`];
      lines.push(`class: ${st.pages[p] ? st.pages[p].toLowerCase() : "—"}`);
      const v = st.verdicts[p];
      if (v) lines.push(v.status === "verified" ? `verified as ${v.class.toLowerCase()}` : `MISMATCH as ${v.class.toLowerCase()}${v.reason ? ": " + v.reason : ""}`);
      const ex = st.extractions[p];
      if (ex) lines.push(ex.ok ? `extracted ok (${ex.class.toLowerCase()})` : `extract FAILED: ${ex.err || ""}`);
      if (st.probes[p]) lines.push(`probing as ${st.probes[p].toLowerCase()}…`);
      if (st.eliminated[p]) lines.push(`ruled out: ${st.eliminated[p].map((c) => c.toLowerCase()).join(", ")}`);
      tip.textContent = lines.join("\n");
      tip.hidden = false;
      tip.style.left = Math.min(e.clientX + 12, innerWidth - 320) + "px";
      tip.style.top = e.clientY + 14 + "px";
    });
    $("strip").addEventListener("mouseleave", () => { tip.hidden = true; });
    $("strip").addEventListener("click", (e) => {
      const cell = e.target.closest(".cell");
      if (!cell) return;
      const p = +cell.dataset.page;
      // jump to the next action (from current) that touches this page, else the first one
      const after = S.model.actions.find((a) => a.seq > S.cur && a.page === p && !a.minor);
      const any = after || S.model.actions.find((a) => a.page === p && !a.minor);
      if (any) seek(any.seq);
    });
  })();

  // ---------- steps list ----------

  function renderSteps(force) {
    const list = $("steps-list");
    const m = S.model;
    const curAct = m.actions[S.cur];
    const gCur = curAct ? curAct.group : 0;
    const G = 12; // groups rendered on each side
    const g0 = Math.max(0, gCur - G);
    const g1 = Math.min(m.groups.length - 1, gCur + G);
    if (!force && g0 === S.stepsWindow.g0 && g1 === S.stepsWindow.g1) {
      highlightSteps();
      return;
    }
    S.stepsWindow = { g0, g1 };
    list.textContent = "";
    const showMinor = $("show-minor").checked;

    const preloop = m.actions.filter((a) => a.group === -1 && (showMinor || !a.minor));
    if (g0 === 0) for (const a of preloop) list.appendChild(stepRow(a));
    if (g0 > 0) {
      const gap = el("div", "gap-note", `… ${g0} earlier iterations — click to jump`);
      gap.onclick = () => { const g = m.groups[Math.max(0, g0 - G)]; if (g.actionSeqs.length) seek(g.actionSeqs[0]); };
      list.appendChild(gap);
    }
    for (let gi = g0; gi <= g1; gi++) {
      const g = m.groups[gi];
      const head = el("div", "group-head");
      head.dataset.group = gi;
      const kindLabel = { committed: "committed", to_deferral: "→ deferral", deferral: "deferral", to_ovstr: "→ stream", ovstr: "override stream" }[g.kind] || g.kind;
      const badge = el("span", "gk " + g.kind, kindLabel);
      head.appendChild(el("span", null, `page ${g.page}`));
      head.appendChild(badge);
      head.onclick = () => { if (g.actionSeqs.length) seek(g.actionSeqs[0]); };
      list.appendChild(head);
      for (const seq of g.actionSeqs) {
        const a = m.actions[seq];
        if (!showMinor && a.minor) continue;
        list.appendChild(stepRow(a));
      }
    }
    if (g1 < m.groups.length - 1) {
      const gap = el("div", "gap-note", `… ${m.groups.length - 1 - g1} later iterations — click to jump`);
      gap.onclick = () => { const g = m.groups[Math.min(m.groups.length - 1, g1 + G)]; if (g.actionSeqs.length) seek(g.actionSeqs[0]); };
      list.appendChild(gap);
    }
    highlightSteps();
  }

  function stepRow(a) {
    const row = el("div", "step-row" + (a.minor ? " minor" : ""));
    if (a.type === "error" || (a.type === "classify_result" && !a.ok)) row.classList.add("err");
    if (a.type === "warn" || (a.type === "extract_result" && !a.ok)) row.classList.add("warnrow");
    row.dataset.seq = a.seq;
    row.appendChild(el("span", "ic", TYPE_ICONS[a.type] || "·"));
    row.appendChild(el("span", "lbl", a.label || a.type));
    row.onclick = () => seek(a.seq);
    return row;
  }

  function highlightSteps() {
    const list = $("steps-list");
    for (const n of list.querySelectorAll(".step-row.current")) n.classList.remove("current");
    for (const n of list.querySelectorAll(".group-head.current")) n.classList.remove("current");
    const row = list.querySelector(`.step-row[data-seq="${S.cur}"]`);
    if (row) {
      row.classList.add("current");
      // manual scroll math (scrollIntoView is unreliable here with sticky
      // group headers + scroll anchoring during playback)
      const target = row.offsetTop - list.clientHeight / 2 + row.offsetHeight / 2;
      const clamped = Math.max(0, Math.min(target, list.scrollHeight - list.clientHeight));
      if (Math.abs(list.scrollTop - clamped) > 8) list.scrollTop = clamped;
    }
    const a = S.model.actions[S.cur];
    if (a && a.group >= 0) {
      const gh = list.querySelector(`.group-head[data-group="${a.group}"]`);
      if (gh) gh.classList.add("current");
    }
  }

  // ---------- detail pane ----------

  function chipEl(cls, opts = {}) {
    const c = el("span", "chip" + (opts.dead ? " dead" : "") + (opts.win ? " win" : ""));
    const sw = el("span", "swatch");
    sw.style.background = colorOf(cls);
    c.appendChild(sw);
    c.appendChild(el("span", null, cls.toLowerCase() + (opts.suffix || "")));
    return c;
  }

  function card(title) {
    const c = el("div", "card");
    if (title) c.appendChild(el("h3", null, title));
    return c;
  }

  function kvList(pairs) {
    const dl = el("dl", "kv");
    for (const [k, v] of pairs) {
      if (v == null || v === "") continue;
      dl.appendChild(el("dt", null, k));
      const dd = el("dd");
      if (v instanceof Node) dd.appendChild(v); else dd.textContent = v;
      dl.appendChild(dd);
    }
    return dl;
  }

  function renderDetail() {
    const body = $("detail-body");
    body.textContent = "";
    if (S.tab === "step") renderStepTab(body);
    else if (S.tab === "tree") renderTreeTab(body);
    else if (S.tab === "margins") renderMarginsTab(body);
    else if (S.tab === "state") renderStateTab(body);
    else if (S.tab === "errors") renderErrorsTab(body);
    else if (S.tab === "extract") renderExtractTab(body);
    else if (S.tab === "final") renderFinalTab(body);
    else if (S.tab === "accuracy") renderAccuracyTab(body);
  }

  function renderStepTab(body) {
    const a = S.model.actions[S.cur];
    if (!a) { body.appendChild(el("p", "dim", "no step selected")); return; }

    const head = card(null);
    const h = el("div", "verdict");
    h.appendChild(el("span", "big", TYPE_ICONS[a.type] || "·"));
    const hh = el("div");
    hh.appendChild(el("div", null, a.label || a.type));
    hh.appendChild(el("div", "dim", `step ${visPos(S.cur) + 1}/${S.visible.length} · log line ${a.line + 1}`));
    h.appendChild(hh);
    head.appendChild(h);
    body.appendChild(head);

    switch (a.type) {
      case "infer": renderInfer(body, a); break;
      case "override_eval": {
        const c = card("tier 4 — override evaluation");
        c.appendChild(kvList([
          ["page", String(a.page)],
          ["matched", a.override.matched || "none"],
          ["action", a.override.action || "no override — inference winner stands"],
        ]));
        body.appendChild(c);
        break;
      }
      case "handle_override": {
        const c = card("override applied");
        const act = a.overrideAction;
        if (act) {
          const explain = {
            Skip: "page is skipped — recorded as UNKNOWN, no classify call is made",
            InferAs: "winner replaced — a classify call validates the override's class",
            ClassifyAs: "classify phase skipped entirely — straight to extraction",
          }[act.kind] || "";
          c.appendChild(kvList([["action", act.kind], ["class", act.class ? chipEl(act.class) : null], ["effect", explain]]));
        }
        body.appendChild(c);
        break;
      }
      case "decide": {
        const c = card("decision recorded");
        c.appendChild(kvList([["page", String(a.page)], ["class", chipEl(a.class)]]));
        c.appendChild(el("p", "dim", "context.decide() — the inference layer commits this guess; ground truth arrives later from the worker pool."));
        body.appendChild(c);
        break;
      }
      case "classify_result": {
        const c = card("classification result (ground truth)");
        const v = el("div", "verdict " + (a.ok ? "ok" : "bad"));
        v.appendChild(el("span", "big", a.ok ? "✓" : "✗"));
        const d = el("div");
        d.appendChild(el("div", null, a.ok
          ? `page ${a.page} confirmed as ${a.class.toLowerCase()}`
          : `page ${a.page} is NOT a ${a.class.toLowerCase()} — inference was wrong`));
        if (a.reason) d.appendChild(el("div", "reason", a.reason));
        if (!a.ok) d.appendChild(el("p", "dim", "a mismatch poisons downstream context → the classifier will request deferral"));
        v.appendChild(d);
        c.appendChild(v);
        body.appendChild(c);
        break;
      }
      case "extract_result": {
        const c = card("extraction result");
        const v = el("div", "verdict " + (a.ok ? "ok" : "warned"));
        v.appendChild(el("span", "big", a.ok ? "⇣" : "▲"));
        const d = el("div");
        d.appendChild(el("div", null, `page ${a.page} · ${a.class.toLowerCase()} · ${a.ok ? "payload extracted" : "extraction FAILED"}`));
        if (a.reason) d.appendChild(el("div", "reason", a.reason));
        v.appendChild(d);
        c.appendChild(v);
        const pay = extractFor(a.page);
        if (pay) {
          c.appendChild(el("div", "dim", "payload streamed to frontend:"));
          const pre = el("pre", "payload", pay.payload);
          c.appendChild(pre);
        }
        body.appendChild(c);
        break;
      }
      case "defer_signal": {
        const c = card("deferral signalled");
        c.appendChild(el("p", null, "A classify job disagreed with inference. The pool is drained, then the state machine leaves Committed mode."));
        body.appendChild(c);
        break;
      }
      case "deferral_begin": {
        const c = card("deferral — find the next anchor");
        c.appendChild(el("p", null,
          `Context after page ${a.page} is untrusted. The classifier probes forward, testing each page against the independent (organizational) classes until one classifies successfully — that page becomes the anchor.`));
        body.appendChild(c);
        break;
      }
      case "probe": {
        const c = card("deferral probe");
        c.appendChild(kvList([["page", String(a.page)], ["try as", chipEl(a.class)]]));
        c.appendChild(el("p", "dim", "an unchecked classify job — success ends the search, failure guarantees this class never matches this page"));
        body.appendChild(c);
        break;
      }
      case "eliminate": {
        const c = card("class ruled out");
        c.appendChild(kvList([["page", String(a.page)], ["not a", chipEl(a.class, { dead: true })]]));
        c.appendChild(el("p", "dim", "recorded in ctx.guarantee_failures — future inference on this page skips this class"));
        body.appendChild(c);
        break;
      }
      case "anchor_found": {
        const c = card("anchor found — deferral ends");
        c.appendChild(kvList([
          ["anchor page", String(a.page)],
          ["anchor class", chipEl(a.class, { win: true })],
          ["deferred region", `${a.fromPage} … ${a.page - 1}`],
        ]));
        c.appendChild(el("p", "dim", "the region behind the anchor is now backfilled structurally (see next steps)"));
        body.appendChild(c);
        break;
      }
      case "fill_strategy": {
        const c = card("backfill strategy");
        const desc = {
          pair: "anchor's only dependents form a pair — deferred pages alternate first/second pair member",
          sole: "anchor has a single dependent class — every deferred page gets it",
          standard: "full inference re-runs over the deferred pages, restricted to the anchor's dependents",
        }[a.strategy];
        c.appendChild(kvList([["strategy", a.strategy], ["pair", a.pair], ["class", a.class ? chipEl(a.class) : null]]));
        if (desc) c.appendChild(el("p", "dim", desc));
        body.appendChild(c);
        break;
      }
      case "ovstr_begin": {
        const c = card("override stream begins");
        c.appendChild(el("p", null, "A multi-page structural pattern was recognized — inference is bypassed while the stream dictates page classes."));
        c.appendChild(el("pre", "payload", a.text));
        body.appendChild(c);
        break;
      }
      case "ovstr_step": {
        const c = card("override stream step");
        c.appendChild(kvList([["page", String(a.page)], ["streamed as", chipEl(a.class)]]));
        body.appendChild(c);
        break;
      }
      case "ovstr_exit": {
        const c = card("override stream exit");
        c.appendChild(el("p", null, `The exit condition classified successfully on page ${a.page} — control returns to committed inference.`));
        body.appendChild(c);
        break;
      }
      case "error": case "warn": {
        const c = card(a.type === "error" ? "error" : "warning");
        c.appendChild(el("div", "reason", a.text));
        body.appendChild(c);
        break;
      }
      case "job_push": {
        const c = card("job queued");
        c.appendChild(kvList([
          ["kind", a.jobtype + (a.unchecked ? " (unchecked — straight to a worker)" : "")],
          ["page", String(a.page)], ["class", chipEl(a.class)],
        ]));
        body.appendChild(c);
        break;
      }
      case "schedule_deferral": case "schedule_ovstr": case "parent_update":
      case "boot": case "fill_begin": default: {
        if (a.text) {
          const c = card(null);
          c.appendChild(el("p", null, a.text));
          body.appendChild(c);
        }
        break;
      }
    }
    renderStateSummary(body);
  }

  function renderInfer(body, a) {
    const inf = a.infer;

    const c0 = card("candidates");
    for (const c of inf.candidates) c0.appendChild(chipEl(c));
    body.appendChild(c0);

    // tier 1
    const c1 = card("tier 1 — definitive");
    const ul = el("ul", "tierlist");
    let defHit = null;
    for (const d of inf.definitive) {
      const li = el("li");
      if (d.passedAll) {
        li.appendChild(el("span", "nomatch", "passed all definitive constraints — no invariant applies"));
      } else if (d.hit) {
        defHit = d;
        li.appendChild(el("span", "cname", d.constraint));
        li.appendChild(el("span", "hit", "  ⚡ HIT — page is definitively "));
        li.appendChild(chipEl(d.winner, { win: true }));
        li.appendChild(el("span", "dim", " (tiers 2–3 skipped)"));
      } else {
        li.appendChild(el("span", "cname", d.constraint));
        li.appendChild(el("span", "nomatch", " — no match"));
      }
      ul.appendChild(li);
    }
    if (!inf.definitive.length) ul.appendChild(el("li", "nomatch", "(not logged at this level)"));
    c1.appendChild(ul);
    body.appendChild(c1);

    if (!defHit) {
      // tier 2 — merge FAIL details with summary rows by constraint
      const c2 = card("tier 2 — hard filters");
      const byConstraint = new Map();
      for (const hRec of inf.hard) {
        const key = hRec.constraint || "?";
        const cur = byConstraint.get(key) || { constraint: key, fails: [], remaining: null };
        if (hRec.failDetail) cur.fails.push(hRec.failDetail);
        if (hRec.remaining) cur.remaining = hRec.remaining;
        byConstraint.set(key, cur);
      }
      const ul2 = el("ul", "tierlist");
      for (const rec of byConstraint.values()) {
        const li = el("li");
        li.appendChild(el("span", "cname", rec.constraint));
        if (rec.fails.length) {
          for (const fd of rec.fails) li.appendChild(el("span", "faildetail", "✗ " + fd));
        } else {
          li.appendChild(el("span", "nomatch", " — nothing eliminated"));
        }
        if (rec.remaining) {
          const rem = el("span", "faildetail");
          rem.style.color = "var(--muted)";
          rem.textContent = "surviving: " + rec.remaining.map((r) => r.toLowerCase()).join(", ");
          li.appendChild(rem);
        }
        ul2.appendChild(li);
      }
      if (inf.hardCounts) {
        ul2.appendChild(el("li", "dim", `candidates: ${inf.hardCounts.before} → ${inf.hardCounts.after}`));
      }
      if (!byConstraint.size) ul2.appendChild(el("li", "nomatch", "(not logged at this level)"));
      c2.appendChild(ul2);
      body.appendChild(c2);

      // tier 3
      const c3 = card("tier 3 — soft scoring");
      if (inf.softSort && inf.softSort.candidates.length) {
        const constraints = [...new Set(inf.softScored.map((s) => s.constraint))];
        if (constraints.length) c3.appendChild(el("p", "dim", "constraints: " + constraints.join(", ")));
        const table = el("table", "scores");
        const thr = el("tr");
        for (const h of ["candidate", "scores", "total", ""]) thr.appendChild(el("th", null, h));
        table.appendChild(thr);
        const maxAbs = Math.max(0.001, ...inf.softSort.candidates.map((c) => Math.abs(c.total ?? 0)));
        const ranked = [...inf.softSort.candidates].reverse(); // log order is ascending
        for (const cand of ranked) {
          const tr = el("tr", cand.class === inf.softSort.winner ? "win" : null);
          const td0 = el("td");
          td0.appendChild(chipEl(cand.class, { win: cand.class === inf.softSort.winner }));
          tr.appendChild(td0);
          tr.appendChild(el("td", null, cand.scores.join(" · ")));
          tr.appendChild(el("td", null, cand.total != null ? cand.total.toFixed(2) : "—"));
          const tdb = el("td", "bar-track");
          const wrap = el("div", "bar-wrap");
          const bar = el("div", "bar");
          bar.style.width = Math.round(Math.abs(cand.total ?? 0) / maxAbs * 110) + "px";
          if ((cand.total ?? 0) < 0) bar.style.background = "var(--critical)";
          wrap.appendChild(bar);
          tdb.appendChild(wrap);
          tr.appendChild(tdb);
          table.appendChild(tr);
        }
        c3.appendChild(table);
      } else {
        c3.appendChild(el("p", "nomatch", inf.candidates.length <= 1
          ? "≤ 1 candidate survived — nothing to rank"
          : "(soft scoring not logged at this level)"));
      }
      body.appendChild(c3);
    }

    const cw = card("winner");
    if (inf.winner) cw.appendChild(chipEl(inf.winner, { win: true }));
    else cw.appendChild(el("span", "nomatch", "unknown"));
    if (inf.winner === "UNKNOWN") cw.appendChild(el("p", "dim", "UNKNOWN — the page is recorded blank, no classify job is queued"));
    body.appendChild(cw);
  }

  function renderStateSummary(body) {
    const st = curState();
    const c = card("state after this step");
    const grid = el("div", "stategrid");
    const left = el("div");
    left.appendChild(kvList([
      ["mode", st.mode],
      ["cursor", st.cursor != null ? "page " + st.cursor : "—"],
      ["current parent", st.parent ? st.parent.toLowerCase() : "(initial)"],
      ["decided pages", String(Object.keys(st.pages).length)],
      ["jobs in flight", String(Object.keys(st.jobs).length)],
    ]));
    grid.appendChild(left);

    const right = el("div");
    const jobs = Object.entries(st.jobs);
    if (jobs.length) {
      right.appendChild(el("h3", null, "in flight"));
      for (const [key, j] of jobs.slice(0, 12)) {
        const [page, kind] = key.split(":");
        right.appendChild(el("div", "dim", `${kind} · page ${page} as ${j.class ? j.class.toLowerCase() : "?"}`));
      }
      if (jobs.length > 12) right.appendChild(el("div", "dim", `… +${jobs.length - 12} more`));
    }
    const elim = Object.entries(st.eliminated);
    if (elim.length) {
      right.appendChild(el("h3", null, "guaranteed failures"));
      for (const [p, arr] of elim) right.appendChild(el("div", "dim", `page ${p}: not ${arr.map((c) => c.toLowerCase()).join(", ")}`));
    }
    grid.appendChild(right);
    c.appendChild(grid);
    body.appendChild(c);
  }

  function renderStateTab(body) {
    renderStateSummary(body);
    const st = curState();
    const c = card("decided pages");
    const t = el("table", "final");
    const thr = el("tr");
    for (const h of ["page", "class", "verdict", "extraction"]) thr.appendChild(el("th", null, h));
    t.appendChild(thr);
    const pages = Object.keys(st.pages).map(Number).sort((a, b) => a - b);
    for (const p of pages) {
      const tr = el("tr");
      tr.appendChild(el("td", null, String(p)));
      const td = el("td");
      td.appendChild(chipEl(st.pages[p]));
      tr.appendChild(td);
      const v = st.verdicts[p];
      tr.appendChild(el("td", v ? (v.status === "verified" ? "okc" : "badc") : "dim",
        v ? (v.status === "verified" ? "✓ verified" : "✗ mismatch") : "pending"));
      const ex = st.extractions[p];
      tr.appendChild(el("td", ex ? (ex.ok ? "okc" : "badc") : "dim", ex ? (ex.ok ? "extracted" : "failed") : "—"));
      t.appendChild(tr);
    }
    c.appendChild(t);
    body.appendChild(c);
  }

  function renderErrorsTab(body) {
    const m = S.model;
    if (!m.errors.length) { body.appendChild(el("p", "dim", "no errors in this run 🎉")); return; }
    for (const seq of m.errors) {
      const a = m.actions[seq];
      const item = el("div", "err-item" + (a.type === "extract_result" ? " warned" : ""));
      const where = el("div", "where");
      const jump = el("span", "jump", `step ${visPos(seq) + 1} · line ${a.line + 1}`);
      jump.onclick = () => { setTab("step"); seek(seq); };
      where.appendChild(jump);
      item.appendChild(where);
      item.appendChild(el("div", null, a.label));
      if (a.reason) item.appendChild(el("div", "reason", a.reason));
      if (a.text) item.appendChild(el("div", "reason", a.text));
      body.appendChild(item);
    }
  }

  function extractFor(page) {
    const list = S.live ? S.live.extracts : S.model.extracts;
    let found = null;
    for (const e of list) if (e.page === page) found = e;
    return found;
  }

  function renderExtractTab(body) {
    const list = S.live ? S.live.extracts : S.model.extracts;
    if (!list.length) {
      body.appendChild(el("p", "dim",
        "no extraction payloads — these are only captured in live-run mode, where the visualizer acts as the classifier's streaming frontend. In file mode see per-page extraction status in the state tab."));
      return;
    }
    for (const e of list) {
      const c = card(`page ${e.page} · ${String(e.class).toLowerCase()}`);
      c.appendChild(el("pre", "payload", e.payload));
      body.appendChild(c);
    }
  }

  // ---------- document tree ----------

  /** Classes acting as independent anchors, inferred from the log itself. */
  function independentClasses() {
    const inds = new Set();
    for (const a of S.model.actions) {
      if (a.type === "probe" || a.type === "anchor_found") inds.add(a.class);
      if (a.effects && a.effects.parent) inds.add(a.effects.parent);
    }
    inds.delete("UNKNOWN");
    inds.delete(null);
    return inds;
  }

  /** Latest decide action for a page at-or-before the current step (for jumps + backroll). */
  function decisionsUpToNow(page) {
    const hist = S.model.pageHistory[page] || [];
    return hist.filter((h) => h.seq <= S.cur);
  }

  function renderTreeTab(body) {
    const st = curState();
    const m = S.model;
    const { fromPage, toPage } = m.meta;
    if (fromPage == null) { body.appendChild(el("p", "dim", "no page range")); return; }

    const inds = independentClasses();
    const rootClass = st.pages[fromPage] && inds.has(st.pages[fromPage])
      ? st.pages[fromPage]
      : [...inds][0] || st.pages[fromPage];

    const defByAnchor = {};
    for (const d of m.deferrals) if (d.anchorPage != null) defByAnchor[d.anchorPage] = d;

    const intro = card(null);
    intro.appendChild(el("p", "dim",
      "the document as currently decided — containers are anchor (organizational) pages; " +
      "⚓ anchors carry the failure branches explored before they were found, ↺ marks pages " +
      "backrolled by a deferral. Click a page to jump to its decision."));
    body.appendChild(intro);

    const rootUl = el("ul", "tree");
    let rootNode = null;   // current root-level container {ul}
    let subNode = null;    // current nested container {ul}

    const containerFor = () => (subNode || rootNode || { ul: rootUl });

    for (let p = fromPage; p < toPage; p++) {
      const cls = st.pages[p];
      if (cls != null && inds.has(cls)) {
        const node = treeContainer(p, cls, st, defByAnchor[p]);
        if (cls === rootClass || !rootNode) {
          rootUl.appendChild(node.li);
          if (cls === rootClass) { rootNode = node; subNode = null; }
          else subNode = node; // e.g. run starts mid-document on a sub-anchor
        } else {
          rootNode.ul.appendChild(node.li);
          subNode = node;
        }
      } else {
        containerFor().ul.appendChild(treeLeaf(p, cls, st));
      }
    }
    const treeCard = card("document tree");
    treeCard.appendChild(rootUl);
    body.appendChild(treeCard);
  }

  function treeBadges(row, p, st) {
    const v = st.verdicts[p];
    if (v) {
      row.appendChild(el("span", "tbadge " + (v.status === "verified" ? "ok" : "bad"),
        v.status === "verified" ? "✓" : "✗ " + v.class.toLowerCase()));
    }
    const ex = st.extractions[p];
    if (ex && !ex.ok) row.appendChild(el("span", "tbadge warned", "▲ extract"));
    const hist = decisionsUpToNow(p);
    if (hist.length > 1) {
      const prev = hist[hist.length - 2].class;
      const b = el("span", "tbadge roll", "↺ was " + prev.toLowerCase());
      b.title = "backrolled: " + hist.map((h) => h.class.toLowerCase()).join(" → ");
      row.appendChild(b);
    }
    if (st.eliminated[p]) {
      row.appendChild(el("span", "tbadge dead", "⊘ not " + st.eliminated[p].map((c) => c.toLowerCase()).join("/")));
    }
    if (st.probes[p]) row.appendChild(el("span", "tbadge probing", "? probing " + st.probes[p].toLowerCase()));
  }

  function treeRowBase(p, cls, st) {
    const row = el("div", "tnode-row" + (st.cursor === p ? " cursorpg" : ""));
    const label = el("span", "tlabel");
    if (cls != null) label.appendChild(chipEl(cls));
    label.appendChild(el("span", "tpage", "page " + p));
    if (cls === "UNKNOWN") label.appendChild(el("span", "dim", "· blank"));
    if (cls == null) label.appendChild(el("span", "undecided", "· unclassified"));
    label.onclick = () => {
      const hist = decisionsUpToNow(p);
      const target = hist.length ? hist[hist.length - 1].seq
        : (S.model.actions.find((a) => a.page === p && !a.minor) || {}).seq;
      if (target != null) { setTab("step"); seek(target); }
    };
    row.appendChild(label);
    treeBadges(row, p, st);
    return row;
  }

  function treeLeaf(p, cls, st) {
    const li = el("li", "tnode leaf" + (cls == null ? " missing" : ""));
    li.appendChild(treeRowBase(p, cls, st));
    return li;
  }

  function treeContainer(p, cls, st, deferral) {
    const li = el("li", "tnode container");
    const row = treeRowBase(p, cls, st);
    const caret = el("span", "caret", S.treeCollapsed.has(p) ? "▸" : "▾");
    row.prepend(caret);

    const ul = el("ul");
    if (S.treeCollapsed.has(p)) ul.hidden = true;
    caret.onclick = (e) => {
      e.stopPropagation();
      if (S.treeCollapsed.has(p)) S.treeCollapsed.delete(p); else S.treeCollapsed.add(p);
      ul.hidden = S.treeCollapsed.has(p);
      caret.textContent = ul.hidden ? "▸" : "▾";
    };

    li.appendChild(row);

    if (deferral && (deferral.anchorSeq == null || deferral.anchorSeq <= S.cur)) {
      const failed = deferral.probes.filter((pr) => pr.ok === false).length + deferral.eliminates.length;
      const btn = el("span", "tbadge anchor", `⚓ anchor · ${failed} failure${failed === 1 ? "" : "s"}`);
      btn.title = "show the branches explored before this anchor was found";
      row.appendChild(btn);
      const panel = failureBranch(deferral);
      panel.hidden = !S.treeFailsOpen.has(p);
      btn.onclick = (e) => {
        e.stopPropagation();
        if (S.treeFailsOpen.has(p)) S.treeFailsOpen.delete(p); else S.treeFailsOpen.add(p);
        panel.hidden = !S.treeFailsOpen.has(p);
      };
      li.appendChild(panel);
    }

    li.appendChild(ul);
    return { li, ul };
  }

  function failureBranch(d) {
    const panel = el("div", "fail-branch");
    panel.appendChild(el("div", "fb-title",
      `deferral from page ${d.fromPage != null ? d.fromPage : "?"} — branches explored`));

    for (const pr of d.probes) {
      const row = el("div", "fb-row" + (pr.ok === false ? " bad" : pr.ok ? " good" : ""));
      const glyph = pr.ok === false ? "✗" : pr.ok ? "⚓" : "…";
      row.appendChild(el("span", "fb-glyph", glyph));
      const txt = el("span", null, `page ${pr.page} as ${pr.class.toLowerCase()}`);
      row.appendChild(txt);
      if (pr.reason) row.appendChild(el("span", "fb-reason", pr.reason));
      const jumpSeq = pr.resSeq != null ? pr.resSeq : pr.seq;
      row.onclick = () => { setTab("step"); seek(jumpSeq); };
      panel.appendChild(row);
    }
    if (d.eliminates.length) {
      const row = el("div", "fb-row dim");
      row.appendChild(el("span", "fb-glyph", "⊘"));
      row.appendChild(el("span", null, "guaranteed failures recorded: " +
        d.eliminates.map((e2) => `${e2.class.toLowerCase()}@${e2.page}`).join(", ")));
      panel.appendChild(row);
    }
    if (d.fills.length) {
      const strat = d.strategy ? d.strategy.strategy : "?";
      const head = el("div", "fb-row roll");
      head.appendChild(el("span", "fb-glyph", "↺"));
      head.appendChild(el("span", null, `backrolled region (strategy: ${strat}):`));
      panel.appendChild(head);
      for (const f of d.fills) {
        const hist = (S.model.pageHistory[f.page] || []).filter((h) => h.seq < f.seq);
        const before = hist.length ? hist[hist.length - 1].class.toLowerCase() : "undecided";
        const row = el("div", "fb-row fill");
        row.appendChild(el("span", "fb-glyph", "·"));
        row.appendChild(el("span", null, `page ${f.page}: ${before} → ${f.class.toLowerCase()}`));
        row.onclick = () => { setTab("step"); seek(f.seq); };
        panel.appendChild(row);
      }
    }
    return panel;
  }

  // ---------- inference margins ----------

  function renderMarginsTab(body) {
    const m = S.model;
    const final = LogModel.stateAt(m, m.actions.length - 1, null);
    const infers = m.actions.filter((a) => a.type === "infer");
    if (!infers.length) { body.appendChild(el("p", "dim", "no inference steps in this log")); return; }

    // outcome + margin per inference
    const items = infers.map((a) => {
      const inf = a.infer;
      const defHit = inf.definitive.some((d) => d.hit);
      let margin = null;
      if (inf.softSort && inf.softSort.candidates.length >= 2) {
        const c = inf.softSort.candidates;
        const top = c[c.length - 1].total, second = c[c.length - 2].total;
        if (top != null && second != null) margin = top - second;
      }
      const v = final.verdicts[a.page];
      const outcome = v && v.class === inf.winner ? (v.status === "verified" ? "correct" : "wrong") : "unchecked";
      return { seq: a.seq, page: a.page, winner: inf.winner, defHit, margin, outcome };
    });

    const scored = items.filter((i) => i.margin != null);
    const wrong = items.filter((i) => i.outcome === "wrong");
    const avg = (xs) => (xs.length ? xs.reduce((s, x) => s + x.margin, 0) / xs.length : null);
    const avgAll = avg(scored);
    const avgWrong = avg(wrong.filter((i) => i.margin != null));

    const intro = card("inference separation");
    intro.appendChild(el("p", "dim",
      "winner-minus-runner-up soft-score margin per inference. Low margins mean the linear " +
      "scoring model barely separated the candidates — when a low bar is also red (ground " +
      "truth disagreed), that inference is a tuning target. Definitive-constraint hits skip " +
      "scoring and show as dots on the top line. Click a bar to jump to the inference."));
    const tiles = el("div", "stategrid");
    const tile = (label, value) => {
      const t = el("div");
      t.appendChild(el("h3", null, label));
      t.appendChild(el("div", "hero", value));
      return t;
    };
    tiles.appendChild(tile("inferences", String(items.length)));
    tiles.appendChild(tile("avg margin", avgAll != null ? avgAll.toFixed(2) : "—"));
    tiles.appendChild(tile("wrong (ground truth)", String(wrong.length)));
    tiles.appendChild(tile("avg margin when wrong", avgWrong != null ? avgWrong.toFixed(2) : "—"));
    intro.appendChild(tiles);
    body.appendChild(intro);

    const c = card("margin per inference");
    const chart = el("div", "margin-chart");
    const maxM = Math.max(0.25, ...scored.map((i) => i.margin));
    const H = 120;
    for (const it of items) {
      const col = el("div", "mcol");
      col.title = `page ${it.page} → ${it.winner ? it.winner.toLowerCase() : "?"}\n` +
        (it.defHit ? "definitive hit (no scoring)" : `margin ${it.margin != null ? it.margin.toFixed(2) : "—"}`) +
        `\noutcome: ${it.outcome}`;
      if (it.defHit) {
        const dot = el("div", "mdot " + it.outcome);
        col.appendChild(dot);
      } else {
        const bar = el("div", "mbar " + it.outcome);
        bar.style.height = Math.max(3, Math.round((it.margin ?? 0) / maxM * H)) + "px";
        col.appendChild(bar);
      }
      col.appendChild(el("div", "mlabel", String(it.page)));
      col.onclick = () => { setTab("step"); seek(it.seq); };
      chart.appendChild(col);
    }
    c.appendChild(chart);
    const lg = el("div", "legend");
    for (const [cls, label, color] of [
      ["correct", "confirmed correct", "var(--good)"],
      ["wrong", "ground truth disagreed", "var(--critical)"],
      ["unchecked", "never validated", "var(--baseline)"],
    ]) {
      const k = el("span", "key");
      const sw = el("span", "swatch");
      sw.style.background = color;
      k.appendChild(sw);
      k.appendChild(el("span", null, label));
      lg.appendChild(k);
    }
    const dk = el("span", "key");
    dk.appendChild(el("span", "glyph", "●"));
    dk.appendChild(el("span", null, "definitive hit"));
    lg.appendChild(dk);
    c.appendChild(lg);
    body.appendChild(c);
  }

  function renderFinalTab(body) {
    const m = S.model;
    const final = LogModel.stateAt(m, m.actions.length - 1, null);
    const stdoutMap = {};
    for (const r of m.results) stdoutMap[r.page] = r.class;
    const { fromPage, toPage } = m.meta;

    const c = card("final structure");
    c.appendChild(el("p", "dim", m.results.length
      ? "every page in the run's range — the binary's stdout vs the replayed decision state. Blank (unknown) and never-decided pages are listed explicitly."
      : "every page in the run's range from the replayed end state (no stdout structure lines in this log). Blank (unknown) and never-decided pages are listed explicitly."));
    const t = el("table", "final");
    const thr = el("tr");
    for (const h of ["page", m.results.length ? "stdout" : "replay", m.results.length ? "replay" : "", ""]) {
      thr.appendChild(el("th", null, h));
    }
    t.appendChild(thr);

    const pages = [];
    if (fromPage != null && toPage != null) {
      for (let p = fromPage; p < toPage; p++) pages.push(p);
    } else {
      pages.push(...new Set([...Object.keys(final.pages).map(Number), ...m.results.map((r) => r.page)]));
      pages.sort((a, b) => a - b);
    }

    for (const p of pages) {
      const out = stdoutMap[p];
      const rep = final.pages[p];
      const tr = el("tr", out == null && rep == null ? "missing-row" : null);
      tr.appendChild(el("td", null, String(p)));
      const td1 = el("td");
      const primary = m.results.length ? out : rep;
      if (primary) td1.appendChild(chipEl(primary));
      else td1.appendChild(el("span", "undecided", "not decided"));
      tr.appendChild(td1);
      const td2 = el("td");
      if (m.results.length) {
        if (rep) td2.appendChild(chipEl(rep));
        else td2.appendChild(el("span", "undecided", "not decided"));
      }
      tr.appendChild(td2);
      const agree = !m.results.length || out === rep || (out == null && rep == null);
      tr.appendChild(el("td", agree ? "okc" : "badc", out == null && rep == null ? "" : agree ? "✓" : "≠"));
      t.appendChild(tr);
    }
    c.appendChild(t);
    body.appendChild(c);
  }

  // ---------- ground-truth accuracy (error %) ----------

  /**
   * Compare the run's final decisions against window.CLASSIFIER_GROUND_TRUTH
   * (generated by build_groundtruth.py from the static-classifier export +
   * PDF text signatures). Uses the binary's stdout structure when present,
   * else the replayed end state — same primary source as the final tab.
   */
  function computeAccuracy() {
    if (S.accuracy) return S.accuracy;
    const gt = window.CLASSIFIER_GROUND_TRUTH;
    const m = S.model;
    if (!gt || !gt.pages || !m) return null;
    const { fromPage, toPage } = m.meta;
    if (fromPage == null || toPage == null) return null;

    const final = LogModel.stateAt(m, m.actions.length - 1, null);
    const stdoutMap = {};
    for (const r of m.results) stdoutMap[r.page] = r.class;
    const useStdout = m.results.length > 0;

    const acc = {
      source: useStdout ? "stdout structure" : "replayed end state",
      compared: 0, wrong: 0, undecided: 0, uncovered: 0,
      mismatches: [],          // {page, truth, decided, anchor, inferred}
      perClass: {},            // TRUTH -> {total, wrong, undecided, conf: {DECIDED: n}}
    };
    for (let p = fromPage; p < toPage; p++) {
      const truth = gt.pages[String(p)];
      if (!truth) { acc.uncovered++; continue; }
      const t = truth.c.toUpperCase();
      const pc = acc.perClass[t] || (acc.perClass[t] = { total: 0, wrong: 0, undecided: 0, conf: {} });
      pc.total++;
      const decidedRaw = useStdout ? stdoutMap[p] : final.pages[p];
      if (decidedRaw == null) { acc.undecided++; pc.undecided++; continue; }
      acc.compared++;
      const d = String(decidedRaw).toUpperCase();
      if (d !== t) {
        acc.wrong++;
        pc.wrong++;
        pc.conf[d] = (pc.conf[d] || 0) + 1;
        acc.mismatches.push({ page: p, truth: t, decided: d, anchor: truth.n || null, inferred: !!truth.i });
      }
    }
    acc.pct = acc.compared ? (acc.wrong / acc.compared) * 100 : null;
    S.accuracy = acc;
    return acc;
  }

  const fmtPct = (x) => (x === 0 ? "0" : x >= 10 ? x.toFixed(0) : x >= 1 ? x.toFixed(1) : x.toFixed(2)) + "%";

  function jumpToPage(p) {
    const after = S.model.actions.find((a) => a.seq > S.cur && a.page === p && !a.minor);
    const any = after || S.model.actions.find((a) => a.page === p && !a.minor);
    if (any) { setTab("step"); seek(any.seq); }
  }

  function renderAccuracyTab(body) {
    const gt = window.CLASSIFIER_GROUND_TRUTH;
    if (!gt || !gt.pages) {
      body.appendChild(el("p", "dim",
        "no ground truth loaded — groundtruth.js is missing. Regenerate it with " +
        "tools/visualizer/build_groundtruth.py (requires pymupdf)."));
      return;
    }
    const acc = computeAccuracy();
    if (!acc) { body.appendChild(el("p", "dim", "no page range in this log yet")); return; }

    const intro = card("classification error vs ground truth");
    intro.appendChild(el("p", "dim",
      `Ground truth for ${gt.doc} pages ${gt.startPage}–${gt.endPage - 1}, derived from the ` +
      `static-classifier export (${gt.source}, generated ${gt.generated}). The run's ` +
      `${acc.source} is compared page-by-page against it.`));

    if (!acc.compared) {
      intro.appendChild(el("p", "nomatch",
        acc.uncovered && !acc.undecided
          ? "this run's page range does not overlap the ground truth"
          : "no decided pages to compare yet"));
      body.appendChild(intro);
      return;
    }

    const tiles = el("div", "stategrid");
    const tile = (label, value, color) => {
      const t = el("div");
      t.appendChild(el("h3", null, label));
      const v = el("div", "hero", value);
      if (color) v.style.color = color;
      t.appendChild(v);
      return t;
    };
    tiles.appendChild(tile("error rate", fmtPct(acc.pct),
      acc.wrong ? "var(--critical)" : "var(--good)"));
    tiles.appendChild(tile("pages compared", String(acc.compared)));
    tiles.appendChild(tile("misclassified", String(acc.wrong), acc.wrong ? "var(--critical)" : null));
    if (acc.undecided) tiles.appendChild(tile("never decided", String(acc.undecided)));
    if (acc.uncovered) tiles.appendChild(tile("no ground truth", String(acc.uncovered)));
    intro.appendChild(tiles);
    body.appendChild(intro);

    // per-class breakdown
    const cc = card("per-class breakdown (by true class)");
    const t = el("table", "final");
    const thr = el("tr");
    for (const h of ["true class", "pages", "wrong", "error", "misclassified as"]) thr.appendChild(el("th", null, h));
    t.appendChild(thr);
    const classes = Object.keys(acc.perClass).sort((a, b) => acc.perClass[b].total - acc.perClass[a].total);
    for (const cls of classes) {
      const pc = acc.perClass[cls];
      const scored = pc.total - pc.undecided;
      const tr = el("tr");
      const td0 = el("td");
      td0.appendChild(chipEl(cls));
      tr.appendChild(td0);
      tr.appendChild(el("td", null, String(pc.total) + (pc.undecided ? ` (${pc.undecided} undecided)` : "")));
      tr.appendChild(el("td", pc.wrong ? "badc" : "okc", String(pc.wrong)));
      tr.appendChild(el("td", pc.wrong ? "badc" : "okc", scored ? fmtPct(pc.wrong / scored * 100) : "—"));
      const confs = Object.entries(pc.conf).sort((a, b) => b[1] - a[1])
        .map(([d, n]) => `${d.toLowerCase()} ×${n}`).join(", ");
      tr.appendChild(el("td", "dim", confs || "—"));
      t.appendChild(tr);
    }
    cc.appendChild(t);
    body.appendChild(cc);

    // mismatched pages
    if (acc.mismatches.length) {
      const cm = card(`misclassified pages (${acc.mismatches.length})`);
      cm.appendChild(el("p", "dim",
        "click a row to jump to the step that touched the page. ° marks pages whose " +
        "ground-truth class was inferred from document position (unreadable OCR caption)."));
      for (const mm of acc.mismatches) {
        const row = el("div", "err-item");
        const where = el("div", "where");
        const jump = el("span", "jump", `page ${mm.page}${mm.inferred ? " °" : ""}`);
        jump.onclick = () => jumpToPage(mm.page);
        where.appendChild(jump);
        row.appendChild(where);
        const line = el("div");
        line.appendChild(el("span", "dim", "truth "));
        line.appendChild(chipEl(mm.truth));
        if (mm.anchor) line.appendChild(el("span", "dim", ` (${mm.anchor})`));
        line.appendChild(el("span", "dim", " · decided "));
        line.appendChild(chipEl(mm.decided));
        row.appendChild(line);
        row.onclick = (e) => { if (e.target === row || e.target === line) jumpToPage(mm.page); };
        cm.appendChild(row);
      }
      body.appendChild(cm);
    } else {
      const cm = card(null);
      cm.appendChild(el("p", "okc", "every compared page matches the ground truth 🎉"));
      body.appendChild(cm);
    }
  }

  // ---------- raw log ----------

  function renderLog() {
    if ($("log-pane").hidden || !S.model) return;
    const a = S.model.actions[S.cur];
    const center = a ? a.line : 0;
    const W = 160;
    const start = Math.max(0, center - W);
    const end = Math.min(S.model.lines.length, center + W);
    const pre = $("log-pre");
    if (start !== S.logWindow.start || end !== S.logWindow.end) {
      S.logWindow = { start, end };
      pre.textContent = "";
      for (let i = start; i < end; i++) {
        const line = el("span", "ln");
        line.dataset.line = i;
        const no = el("span", "lno", String(i + 1).padStart(6) + "  ");
        line.appendChild(no);
        line.appendChild(document.createTextNode(S.model.lines[i]));
        line.onclick = () => {
          const hitExact = S.model.actions.find((x) => x.line === i);
          const hit = hitExact || [...S.model.actions].reverse().find((x) => x.line <= i);
          if (hit) seek(hit.seq);
        };
        pre.appendChild(line);
      }
    }
    for (const n of pre.querySelectorAll(".ln.hl")) n.classList.remove("hl");
    if (a) {
      let target = null;
      for (let i = a.line; i <= (a.lineEnd ?? a.line); i++) {
        const n = pre.querySelector(`.ln[data-line="${i}"]`);
        if (n) { n.classList.add("hl"); if (!target) target = n; }
      }
      if (target) target.scrollIntoView({ block: "center" });
    }
  }

  // ---------- seek / playback ----------

  function curState() {
    return LogModel.stateAt(S.model, S.cur, S.stateCache);
  }

  function seek(seq, opts = {}) {
    if (!S.model) return;
    seq = Math.max(0, Math.min(seq, S.model.actions.length - 1));
    if (seq === S.cur && !opts.force) return;
    // backward seek invalidates the incremental cache
    if (S.stateCache.idx != null && seq < S.stateCache.idx) S.stateCache = { idx: null, state: null };
    S.cur = seq;
    const st = curState();
    updateStrip(st, S.model.actions[seq]);
    renderSteps(opts.force);
    renderDetail();
    renderLog();

    const a = S.model.actions[seq];
    const g = a && a.group >= 0 ? S.model.groups[a.group] : null;
    $("pb-scrub").value = String(visPos(seq));
    $("pb-label").textContent =
      `step ${visPos(seq) + 1}/${S.visible.length}` + (g ? ` · pg ${g.page} ${g.kind}` : "");
    $("step-count").textContent = `${S.visible.length}`;
    $("err-count").textContent = S.model.errors.length ? String(S.model.errors.length) : "";
    const extracts = S.live ? S.live.extracts : S.model.extracts;
    $("ext-count").textContent = extracts.length ? String(extracts.length) : "";
    const acc = computeAccuracy();
    $("acc-pct").textContent = acc && acc.compared ? fmtPct(acc.pct) : "";
    const badge = $("mode-badge");
    badge.textContent = st.mode.replace("_", " ");
    badge.className = "mode-badge " + st.mode;
  }

  function stepVisible(delta) {
    const pos = visPos(S.cur);
    const next = Math.max(0, Math.min(S.visible.length - 1, pos + delta));
    seek(S.visible[next]);
  }

  function stepGroup(delta) {
    const a = S.model.actions[S.cur];
    const g = Math.max(0, Math.min(S.model.groups.length - 1, (a ? a.group : 0) + delta));
    const grp = S.model.groups[g];
    if (grp && grp.actionSeqs.length) seek(grp.actionSeqs[0]);
  }

  function togglePlay(force) {
    const on = force != null ? force : !S.playTimer;
    if (S.playTimer) { clearInterval(S.playTimer); S.playTimer = null; }
    if (on) {
      S.playTimer = setInterval(() => {
        const pos = visPos(S.cur);
        if (pos >= S.visible.length - 1) { togglePlay(false); return; }
        seek(S.visible[pos + 1]);
      }, 150);
    }
    $("pb-play").textContent = S.playTimer ? "⏸" : "▶";
  }

  // ---------- live run ----------

  async function checkServer() {
    try {
      const r = await fetch("api/defaults");
      if (!r.ok) throw 0;
      const d = await r.json();
      S.serverOk = true;
      $("live-btn").hidden = false;
      $("lf-doc").value = d.doc || "";
      if (!d.exe_exists) $("live-btn").title = "classifier binary not found — build it first";
    } catch { /* file:// or plain static hosting */ }
    try {
      const r = await fetch("sample/sample_run.log", { method: "HEAD" });
      if (r.ok) $("sample-btn").hidden = false;
    } catch { }
  }

  async function startLive(ev) {
    ev.preventDefault();
    $("live-error").textContent = "";
    const body = {
      doc: $("lf-doc").value.trim(),
      start: +$("lf-start").value,
      end: +$("lf-end").value,
      threads: +$("lf-threads").value,
      level: $("lf-level").value,
    };
    try {
      const r = await fetch("api/run", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const d = await r.json();
      if (!r.ok) throw new Error(d.error || r.statusText);
      attachRun(d.id);
      $("live-panel").hidden = true;
    } catch (e) {
      $("live-error").textContent = String(e.message || e);
    }
  }

  function attachRun(id) {
    if (S.live && S.live.es) S.live.es.close();
    S.live = { id, es: null, stderr: [], stdout: [], extracts: [], status: "running" };
    S.cur = -1;
    $("run-status").hidden = false;
    setRunStatus("running", "run #" + id + " — running");
    $("stop-btn").hidden = false;
    $("follow-wrap").hidden = false;

    const es = new EventSource(`api/run/${id}/events`);
    S.live.es = es;
    es.onmessage = (m) => {
      let e;
      try { e = JSON.parse(m.data); } catch { return; }
      if (e.type === "stderr") S.live.stderr.push(e.data);
      else if (e.type === "stdout") S.live.stdout.push(e.data);
      else if (e.type === "extract") S.live.extracts.push(e);
      else if (e.type === "status") {
        S.live.status = e.status;
        setRunStatus(e.status === "done" ? "done" : (e.status === "running" ? "running" : "failed"),
          `run #${id} — ${e.status}${e.code != null ? " (exit " + e.code + ")" : ""}`);
        if (e.status !== "running") { es.close(); $("stop-btn").hidden = true; scheduleReparse(true); }
      }
      if (e.type === "stderr" || e.type === "stdout") scheduleReparse(false);
    };
    es.onerror = () => { /* server gone or run over; keep what we have */ };
  }

  function setRunStatus(cls, text) {
    const n = $("run-status");
    n.className = "run-status " + cls;
    n.textContent = text;
  }

  function scheduleReparse(immediate) {
    if (S.reparseTimer) return;
    S.reparseTimer = setTimeout(() => {
      S.reparseTimer = null;
      if (!S.live) return;
      const text = S.live.stderr.join("\n") + "\n" + S.live.stdout.join("\n");
      if (!text.trim()) return;
      const follow = $("follow").checked;
      loadText(text, { extracts: S.live.extracts, keepPosition: !follow, goToEnd: follow });
    }, immediate ? 10 : 400);
  }

  async function stopLive() {
    if (!S.live) return;
    try { await fetch(`api/run/${S.live.id}/stop`, { method: "POST" }); } catch { }
  }

  // ---------- wiring ----------

  $("file-input").addEventListener("change", async (e) => {
    const f = e.target.files[0];
    if (!f) return;
    detachLive();
    loadText(await f.text());
    e.target.value = "";
  });

  $("paste-btn").onclick = () => $("paste-dialog").showModal();
  $("paste-dialog").addEventListener("close", () => {
    if ($("paste-dialog").returnValue === "ok") {
      const t = $("paste-text").value;
      if (t.trim()) { detachLive(); loadText(t); }
    }
  });

  $("sample-btn").onclick = async () => {
    const r = await fetch("sample/sample_run.log");
    detachLive();
    loadText(await r.text());
  };

  $("live-btn").onclick = () => { $("live-panel").hidden = !$("live-panel").hidden; };
  $("live-form").addEventListener("submit", startLive);
  $("stop-btn").onclick = stopLive;

  function detachLive() {
    if (S.live && S.live.es) S.live.es.close();
    S.live = null;
    $("run-status").hidden = true;
    $("stop-btn").hidden = true;
    $("follow-wrap").hidden = true;
  }

  $("show-minor").addEventListener("change", () => {
    if (!S.model) return;
    rebuildVisible();
    S.stepsWindow = { g0: -1, g1: -1 };
    seek(S.cur < 0 ? firstVisible() : S.cur, { force: true });
  });

  $("zoom").addEventListener("input", () => {
    if (!S.model) return;
    buildStrip();
    seek(S.cur, { force: true });
  });

  $("theme-btn").onclick = () => {
    const cur = document.documentElement.dataset.theme || "auto";
    const next = cur === "auto" ? "dark" : cur === "dark" ? "light" : "auto";
    if (next === "auto") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = next;
    localStorage.setItem("viz-theme", next);
  };
  const savedTheme = localStorage.getItem("viz-theme");
  if (savedTheme && savedTheme !== "auto") document.documentElement.dataset.theme = savedTheme;

  for (const t of document.querySelectorAll("#detail-tabs .tab")) {
    t.onclick = () => setTab(t.dataset.tab);
  }
  function setTab(name) {
    S.tab = name;
    for (const t of document.querySelectorAll("#detail-tabs .tab")) {
      t.classList.toggle("on", t.dataset.tab === name);
    }
    renderDetail();
  }

  $("pb-first").onclick = () => seek(firstVisible());
  $("pb-last").onclick = () => seek(lastVisible());
  $("pb-prev").onclick = () => stepVisible(-1);
  $("pb-next").onclick = () => stepVisible(1);
  $("pb-prev-group").onclick = () => stepGroup(-1);
  $("pb-next-group").onclick = () => stepGroup(1);
  $("pb-play").onclick = () => togglePlay();
  $("pb-scrub").addEventListener("input", (e) => {
    const pos = +e.target.value;
    if (S.visible[pos] != null) seek(S.visible[pos]);
  });

  $("log-toggle").onclick = () => {
    const p = $("log-pane");
    p.hidden = !p.hidden;
    S.logWindow = { start: -1, end: -1 };
    renderLog();
  };

  document.addEventListener("keydown", (e) => {
    if (!S.model) return;
    if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA" || e.target.tagName === "SELECT") return;
    switch (e.key) {
      case "ArrowLeft": e.shiftKey ? stepGroup(-1) : stepVisible(-1); e.preventDefault(); break;
      case "ArrowRight": e.shiftKey ? stepGroup(1) : stepVisible(1); e.preventDefault(); break;
      case "Home": seek(firstVisible()); e.preventDefault(); break;
      case "End": seek(lastVisible()); e.preventDefault(); break;
      case " ": togglePlay(); e.preventDefault(); break;
    }
  });

  checkServer();

  // deep link: ?load=sample loads the bundled sample run on startup
  if (new URLSearchParams(location.search).get("load") === "sample") {
    fetch("sample/sample_run.log")
      .then((r) => r.text())
      .then((t) => loadText(t))
      .catch(() => { });
  }
})();
