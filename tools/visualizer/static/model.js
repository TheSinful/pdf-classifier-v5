/**
 * model.js — turns the parsed log tree (parser.js) into a linear sequence of
 * semantic *actions* (the things you step through) grouped by classifier-loop
 * iteration, each with an `effects` object that can be replayed to reconstruct
 * the document state (page → class map, verifications, mismatches, eliminations,
 * mode, current parent, jobs in flight) at any point in time.
 */
(function (global) {
  "use strict";

  const P = typeof module !== "undefined" && module.exports ? require("./parser.js") : global.LogParser;
  const { pageNum, className, parseResValue, splitTopLevel } = P;

  const SCORE_VALUES = {
    REWARD_Heavy: 1.0,
    REWARD_Light: 0.5,
    Neutral: 0.0,
    PUNISHMENT_Light: -0.5,
    PUNISHMENT_Heavy: -1.0,
  };

  function scoreValue(name) {
    if (name in SCORE_VALUES) return SCORE_VALUES[name];
    const m = /^Custom\((-?[\d.]+)\)$/.exec(name);
    return m ? parseFloat(m[1]) : null;
  }

  /** Parse the "soft sort complete" trace message into ranked candidates. */
  function parseSoftSort(msg) {
    const winner = (/top candidate is ([\w-]+)/.exec(msg) || [])[1] || null;
    const listMatch = /\(all candidates\)\s*\[(.*)\]\s*$/s.exec(msg);
    const candidates = [];
    if (listMatch) {
      for (const item of splitTopLevel(listMatch[1])) {
        const m = /^\(([\w-]+),\s*\[(.*)\]\)$/s.exec(item);
        if (!m) continue;
        const scores = splitTopLevel(m[2]).filter((s) => s.length);
        const values = scores.map(scoreValue);
        const total = values.every((v) => v !== null)
          ? values.reduce((a, b) => a + b, 0)
          : null;
        candidates.push({ class: className(m[1]), scores, total });
      }
    }
    return { winner: winner ? className(winner) : null, candidates };
  }

  /** Fold an `infer` span's children into one structured record. */
  function foldInfer(span) {
    const rec = {
      page: pageNum(span.fields.page),
      candidates: [],
      definitive: [],
      hard: [],
      softScored: [],
      softSort: null,
      winner: null,
      error: null,
    };
    const candM = /^\[(.*)\]$/s.exec(span.fields.candidates || "");
    if (candM) rec.candidates = splitTopLevel(candM[1]).map(className);

    (function visit(node) {
      for (const ch of node.children) {
        if (ch.kind === "event") {
          const f = ch.fields;
          if (ch.message.includes("class hit definitive constraint")) {
            rec.definitive.push({ constraint: f.hit_constraint, hit: true, winner: className(f.winner) });
          } else if (ch.message.includes("no match on definitive constraint")) {
            rec.definitive.push({ constraint: f.def_constraint, hit: false });
          } else if (ch.message.includes("passed all definitive constraints")) {
            rec.definitive.push({ constraint: null, hit: false, passedAll: true });
          } else if (ch.message.includes("failed hard constraint") || f.hard_constraint) {
            rec.hard.push({
              constraint: f.hard_constraint,
              remaining: (/^\[(.*)\]$/s.exec(f.candidates_remaining || "") || [null, ""])[1]
                .split(",").map((s) => className(s)).filter((s) => s && s.length),
            });
          } else if (ch.message.includes("hard filtering done")) {
            const m = /(\d+)\s*->\s*(\d+)/.exec(ch.message);
            if (m) rec.hardCounts = { before: +m[1], after: +m[2] };
          } else if (ch.message.includes("soft sort complete")) {
            rec.softSort = parseSoftSort(ch.message);
          } else if (ch.message.startsWith("FAIL (")) {
            const m = /^FAIL \((\w+)\):\s*(.*)$/s.exec(ch.message);
            rec.hard.push({ constraint: m ? m[1] : null, failDetail: m ? m[2] : ch.message });
          } else if (f.return != null) {
            rec.winner = className(f.return);
          } else if (ch.level === "ERROR") {
            rec.error = ch.message || JSON.stringify(f);
          }
        } else if (ch.kind === "span" && ch.name === "soft_constraint_score") {
          rec.softScored.push({
            constraint: ch.fields.constraint,
            class: className(ch.fields.class),
            page: pageNum(ch.fields.page),
          });
        }
        visit(ch);
      }
    })(span);
    return rec;
  }

  /** Fold `_override` span into {matched, action}. */
  function foldOverride(span) {
    const rec = { page: pageNum(span.fields.page), matched: null, action: null };
    (function visit(node) {
      for (const ch of node.children) {
        if (ch.kind === "event") {
          if (ch.fields.override) rec.matched = ch.fields.override;
          if (ch.fields.return != null) {
            const v = ch.fields.return;
            rec.action = v === "None" ? null : (/^Some\((.*)\)$/s.exec(v) || [null, v])[1];
          }
        }
        visit(ch);
      }
    })(span);
    return rec;
  }

  /** Parse OverrideAction strings: Skip | InferAs(X) | ClassifyAs(X). */
  function parseOverrideAction(v) {
    if (v == null) return null;
    const s = String(v).trim();
    if (s === "Skip") return { kind: "Skip", class: "UNKNOWN" };
    let m = /^InferAs\((\w+)\)$/.exec(s);
    if (m) return { kind: "InferAs", class: className(m[1]) };
    m = /^ClassifyAs\((\w+)\)$/.exec(s);
    if (m) return { kind: "ClassifyAs", class: className(m[1]) };
    return { kind: s, class: null };
  }

  /**
   * Build the model.
   * @param parsed output of LogParser.parseLog
   * @param extra  optional { extracts: [{page, class, payload}] } from a live run
   */
  function buildModel(parsed, extra) {
    const actions = [];
    const groups = [];
    const errors = [];

    let meta = { fromPage: null, toPage: null };
    let curGroup = null;

    function emit(a) {
      a.seq = actions.length;
      a.group = curGroup ? curGroup.idx : -1;
      if (a.minor == null) a.minor = false;
      if (!a.effects) a.effects = {};
      actions.push(a);
      if (curGroup) curGroup.actionSeqs.push(a.seq);
      if (a.type === "error" || a.effects.mismatch || (a.effects.extract && !a.effects.extract.ok)) {
        errors.push(a.seq);
      }
      return a;
    }

    function emitEvent(ev) {
      const msg = ev.message || "";
      const f = ev.fields;
      const base = { line: ev.line, lineEnd: ev.lineEnd, level: ev.level };

      if (ev.level === "ERROR") {
        return emit({ ...base, type: "error", label: "ERROR", text: msg || JSON.stringify(f) });
      }
      if (ev.level === "WARN") {
        return emit({ ...base, type: "warn", label: "WARN", text: msg || JSON.stringify(f) });
      }
      let m;
      if ((m = /current parent updated to ([\w-]+)/.exec(msg))) {
        return emit({
          ...base, type: "parent_update", label: "parent → " + className(m[1]),
          text: msg, effects: { parent: className(m[1]) },
        });
      }
      if ((m = /\[OVSTR\] streaming page (\d+) as class ([\w-]+)/.exec(msg))) {
        return emit({
          ...base, type: "ovstr_step", page: +m[1], class: className(m[2]),
          label: `stream page ${m[1]} as ${className(m[2])}`, text: msg,
          effects: { cursor: +m[1] },
        });
      }
      if ((m = /beginning override for: (.*)$/s.exec(msg))) {
        return emit({ ...base, type: "ovstr_begin", label: "override stream begins", text: m[1] });
      }
      if ((m = /evaluating for exit case on page (\d+)/.exec(msg))) {
        return emit({ ...base, type: "ovstr_exit_check", page: +m[1], minor: true, label: `exit-probe page ${m[1]}`, text: msg });
      }
      if ((m = /hit exit case for stream on page (\d+)/.exec(msg))) {
        return emit({
          ...base, type: "ovstr_exit", page: +m[1], label: `stream exit on page ${m[1]}`,
          text: msg, effects: { mode: "COMMITTED" },
        });
      }
      if ((m = /initialized context with page range: \[(\d+),(\d+)\]/.exec(msg))) {
        meta.fromPage = +m[1];
        meta.toPage = +m[2];
        return emit({ ...base, type: "boot", label: "context initialized", text: msg });
      }
      if (msg.includes("initialized threadpool")) {
        const w = /with (\d+) workers/.exec(msg);
        return emit({
          ...base, type: "boot", label: "thread pool up", text: msg,
          effects: w ? { workers: +w[1] } : {},
        });
      }
      // everything else: keep, but minor
      return emit({ ...base, type: "note", minor: true, label: msg.slice(0, 80) || ev.level, text: msg });
    }

    function walk(node) {
      for (const ch of node.children) {
        if (ch.kind === "event" || ch.kind === "raw") {
          emitEvent(ch);
          continue;
        }
        const f = ch.fields;
        const base = { line: ch.line, lineEnd: ch.lineEnd };
        switch (ch.name) {
          case "start_classifiation_loop": {
            if (meta.fromPage == null) meta.fromPage = pageNum(f.from_page);
            if (meta.toPage == null) meta.toPage = pageNum(f.to_page);
            walk(ch);
            break;
          }
          case "classification": {
            curGroup = {
              idx: groups.length,
              page: pageNum(f.page),
              kind: "committed",
              label: `page ${f.page}`,
              line: ch.line,
              actionSeqs: [],
            };
            // classify the iteration by its direct children
            for (const c of ch.children) {
              if (c.kind !== "span") continue;
              if (c.name === "schedule_deferral") curGroup.kind = "to_deferral";
              else if (c.name === "enter_deferral") curGroup.kind = "deferral";
              else if (c.name === "schedule_override_stream") curGroup.kind = "to_ovstr";
              else if (c.name === "exit_override_stream") curGroup.kind = "ovstr";
            }
            groups.push(curGroup);
            walk(ch);
            curGroup = null;
            break;
          }
          case "step":
            emit({ ...base, type: "step_begin", page: pageNum(f.current_page), minor: true,
                   label: `step page ${f.current_page}`, effects: { cursor: pageNum(f.current_page) } });
            walk(ch);
            break;
          case "infer": {
            const rec = foldInfer(ch);
            const a = emit({
              ...base, type: "infer", page: rec.page, infer: rec, class: rec.winner,
              lineEnd: maxLine(ch),
              label: `infer page ${rec.page} → ${rec.winner || "?"}`,
              effects: { cursor: rec.page },
            });
            if (rec.error) emit({ ...base, type: "error", label: "inference error", text: rec.error });
            break;
          }
          case "_override": {
            const rec = foldOverride(ch);
            emit({
              ...base, type: "override_eval", page: rec.page, override: rec,
              lineEnd: maxLine(ch), minor: rec.action == null,
              label: rec.action == null ? "no override" : `override: ${rec.matched || ""} → ${rec.action}`,
            });
            break;
          }
          case "handle_override": {
            const act = parseOverrideAction(f.override_result);
            emit({
              ...base, type: "handle_override", overrideAction: act,
              label: `apply override ${f.override_result}`,
            });
            walk(ch);
            break;
          }
          case "decide_as": {
            const page = pageNum(f.page);
            const cls = className(f.class);
            emit({
              ...base, type: "decide", page, class: cls,
              label: `decide page ${page} = ${cls}`,
              effects: { assign: { page, class: cls } },
            });
            walk(ch);
            break;
          }
          case "decide_and_classify_as":
            walk(ch); // children: decide_as + push_classification_job
            break;
          case "increment_current_page":
            emit({
              ...base, type: "advance", minor: true,
              label: `advance → page ${f.to_page}`,
              effects: { cursor: pageNum(f.to_page) },
            });
            break;
          case "push_classification_job":
          case "push_unchecked_classification_job":
          case "push_extraction_job": {
            const jobtype = ch.name === "push_extraction_job" ? "extract" : "classify";
            const page = pageNum(f.page);
            const cls = className(f.class);
            emit({
              ...base, type: "job_push", jobtype, page, class: cls,
              unchecked: ch.name.includes("unchecked"),
              lineEnd: maxLine(ch), minor: true,
              label: `queue ${jobtype} ${cls} @ page ${page}`,
              effects: {
                jobStart: { page, class: cls, jobtype },
                workers: f.available_workers != null ? +f.available_workers : undefined,
              },
            });
            break;
          }
          case "work_available_worker":
            emit({ ...base, type: "worker_assign", minor: true, label: `worker ${f.worker} takes a job` });
            break;
          case "polled_results":
            emit({ ...base, type: "poll", minor: true, label: `poll: ${f.result_count} result(s)` });
            walk(ch);
            break;
          case "handle_classification_result": {
            const page = pageNum(f.page);
            const cls = className(f.class);
            const res = parseResValue(f.res) || { ok: true, reason: null };
            emit({
              ...base, type: "classify_result", page, class: cls, ok: res.ok, reason: res.reason,
              label: res.ok
                ? `✓ classify ${cls} @ page ${page} confirmed`
                : `✗ classify ${cls} @ page ${page} FAILED`,
              effects: res.ok
                ? { verify: { page, class: cls }, jobEnd: { page, jobtype: "classify" } }
                : { mismatch: { page, class: cls, reason: res.reason }, jobEnd: { page, jobtype: "classify" } },
            });
            walk(ch);
            break;
          }
          case "handle_extraction_result": {
            const page = pageNum(f.page);
            const cls = className(f.class);
            const res = parseResValue(f.res) || { ok: true, reason: null };
            emit({
              ...base, type: "extract_result", page, class: cls, ok: res.ok, reason: res.reason,
              minor: res.ok,
              label: res.ok
                ? `extract ${cls} @ page ${page} ok`
                : `✗ extract ${cls} @ page ${page} FAILED`,
              effects: { extract: { page, class: cls, ok: res.ok, err: res.reason }, jobEnd: { page, jobtype: "extract" } },
            });
            walk(ch);
            break;
          }
          case "signal_deferral":
            emit({
              ...base, type: "defer_signal", page: pageNum(f.page),
              label: `mismatch → deferral requested (queue: ${f.tasks_in_queue})`,
              effects: { deferSignal: true },
            });
            walk(ch);
            break;
          case "schedule_deferral":
            emit({
              ...base, type: "schedule_deferral", page: pageNum(f.page),
              label: `drain pool, switch to DEFERRAL @ page ${f.page}`,
              effects: { mode: "DEFERRAL" },
            });
            walk(ch);
            break;
          case "schedule_override_stream":
            emit({
              ...base, type: "schedule_ovstr", page: pageNum(f.page),
              label: `switch to OVERRIDE STREAM @ page ${f.page}`,
              effects: { mode: "OVERRIDE_STREAM" },
            });
            walk(ch);
            break;
          case "enter_deferral":
            emit({
              ...base, type: "deferral_begin", page: pageNum(f.page),
              label: `deferral search from page ${f.page}`,
              effects: { mode: "DEFERRAL", deferFrom: pageNum(f.page) },
            });
            walk(ch);
            break;
          case "check_independent": {
            const page = pageNum(f.page);
            emit({
              ...base, type: "probe", page, class: className(f.independent),
              lineEnd: maxLine(ch),
              label: `probe page ${page} as ${className(f.independent)}?`,
              effects: { probe: { page, class: className(f.independent) }, cursor: page },
            });
            break;
          }
          case "eliminate_class": {
            const page = pageNum(f.page);
            const cls = className(f.class);
            emit({
              ...base, type: "eliminate", page, class: cls,
              label: `✗ page ${page} is NOT ${cls} (guaranteed)`,
              effects: { eliminate: { page, class: cls }, jobEnd: { page, jobtype: "classify" } },
            });
            walk(ch);
            break;
          }
          case "exit_deferral": {
            const page = pageNum(f.on_page);
            const cls = className(f.as_class);
            emit({
              ...base, type: "anchor_found", page, class: cls, fromPage: pageNum(f.from_page),
              label: `⚓ anchor found: page ${page} = ${cls}`,
              effects: { anchor: { page, class: cls, from: pageNum(f.from_page) } },
            });
            walk(ch);
            break;
          }
          case "fill_in_dependents":
            emit({
              ...base, type: "fill_begin", class: className(f.ended_on_class), minor: true,
              label: `backfill deferred pages (anchor ${className(f.ended_on_class)})`,
            });
            walk(ch);
            break;
          case "fill_in_with_only_pair":
            emit({ ...base, type: "fill_strategy", strategy: "pair", pair: f.pair, label: `backfill strategy: alternate pair ${f.pair || ""}` });
            walk(ch);
            break;
          case "fill_in_with_sole_class":
            emit({ ...base, type: "fill_strategy", strategy: "sole", class: className(f.class), label: `backfill strategy: sole class ${className(f.class)}` });
            walk(ch);
            break;
          case "fill_in_by_standard_classification":
            emit({ ...base, type: "fill_strategy", strategy: "standard", label: "backfill strategy: standard inference over dependents" });
            walk(ch);
            break;
          case "deferral_fill_range":
            emit({ ...base, type: "fill_range", minor: true, label: `fill range [${f.from_page}..${f.to_page}]` });
            walk(ch);
            break;
          case "exit_override_stream":
            emit({
              ...base, type: "ovstr_container", minor: true, label: "override stream run",
              effects: { mode: "OVERRIDE_STREAM" },
            });
            walk(ch);
            break;
          default:
            emit({ ...base, type: "span", name: ch.name, minor: true, label: ch.name });
            walk(ch);
            break;
        }
      }
    }

    function maxLine(node) {
      let max = node.lineEnd != null ? node.lineEnd : node.line;
      for (const c of node.children) max = Math.max(max, maxLine(c));
      return max;
    }

    walk(parsed.root);

    // Deferral end bookkeeping: after a deferral group's last action, mode returns
    // to COMMITTED and the region becomes a permanent "was deferred" band.
    for (const g of groups) {
      if (g.kind !== "deferral" || !g.actionSeqs.length) continue;
      const gActs = g.actionSeqs.map((s) => actions[s]);
      const anchor = gActs.find((a) => a.type === "anchor_found");
      // attach the mode restore to the last non-minor action so the stepper's
      // default (micro-steps hidden) view sees the transition too
      const last = [...gActs].reverse().find((a) => !a.minor) || gActs[gActs.length - 1];
      if (anchor) {
        last.effects = last.effects || {};
        last.effects.mode = "COMMITTED";
        last.effects.deferRegionDone = { from: anchor.effects.anchor.from, to: anchor.page };
      }
    }

    // Per-page decision history (a page decided more than once was "backrolled",
    // e.g. re-decided during a deferral backfill).
    const pageHistory = {};
    for (const a of actions) {
      if (a.type === "decide") {
        (pageHistory[a.page] = pageHistory[a.page] || []).push({ seq: a.seq, class: a.class });
      }
    }

    // Per-deferral failure record: every probe with its eventual outcome (the
    // classify result may arrive many actions later, even after the anchor was
    // finalized), plus eliminations and the backfill decisions.
    const deferrals = [];
    for (const g of groups) {
      if (g.kind !== "deferral") continue;
      const acts = g.actionSeqs.map((s) => actions[s]);
      const anchor = acts.find((a) => a.type === "anchor_found");
      const probes = acts
        .filter((a) => a.type === "probe")
        .map((p) => {
          const res = actions.find(
            (x) => x.type === "classify_result" && x.page === p.page && x.class === p.class && x.seq > p.seq
          );
          const rec = {
            seq: p.seq, page: p.page, class: p.class,
            ok: res ? res.ok : null,
            reason: res ? res.reason : null,
            resSeq: res ? res.seq : null,
          };
          // the winning probe's Ok result is consumed by the anchor search itself
          // and never reaches handle_classification_result — credit it via the anchor
          if (rec.ok == null && anchor && p.page === anchor.page && p.class === anchor.class) {
            rec.ok = true;
            rec.resSeq = anchor.seq;
          }
          return rec;
        });
      deferrals.push({
        group: g.idx,
        beginSeq: acts.length ? acts[0].seq : null,
        anchorSeq: anchor ? anchor.seq : null,
        anchorPage: anchor ? anchor.page : null,
        anchorClass: anchor ? anchor.class : null,
        fromPage: anchor ? anchor.fromPage : null,
        probes,
        eliminates: acts.filter((a) => a.type === "eliminate"),
        fills: acts.filter((a) => a.type === "decide" && (!anchor || a.page !== anchor.page)),
        strategy: acts.find((a) => a.type === "fill_strategy") || null,
      });
    }

    return {
      meta,
      groups,
      actions,
      errors,
      results: parsed.results,
      lines: parsed.lines,
      extracts: (extra && extra.extracts) || [],
      pageHistory,
      deferrals,
    };
  }

  // ---------- state replay ----------

  function initialState(meta) {
    return {
      pages: {},          // page -> class
      verdicts: {},       // page -> {status: 'verified'|'mismatch', class, reason}
      extractions: {},    // page -> {class, ok, err}
      eliminated: {},     // page -> [classes]
      probes: {},         // page -> class currently being probed (deferral)
      jobs: {},           // "page:jobtype" -> {class}
      mode: "COMMITTED",
      parent: null,
      cursor: meta.fromPage,
      workers: null,
      deferFrom: null,
      deferRegions: [],   // [{from,to}] completed deferral regions
      anchor: null,
    };
  }

  function applyEffects(st, a) {
    const e = a.effects || {};
    if (e.assign) {
      st.pages[e.assign.page] = e.assign.class;
      // a re-decision clears a stale verdict for that page
      const v = st.verdicts[e.assign.page];
      if (v && v.class !== e.assign.class) delete st.verdicts[e.assign.page];
    }
    if (e.verify) st.verdicts[e.verify.page] = { status: "verified", class: e.verify.class, reason: null };
    if (e.mismatch) st.verdicts[e.mismatch.page] = { status: "mismatch", class: e.mismatch.class, reason: e.mismatch.reason };
    if (e.extract) st.extractions[e.extract.page] = { class: e.extract.class, ok: e.extract.ok, err: e.extract.err };
    if (e.eliminate) {
      (st.eliminated[e.eliminate.page] = st.eliminated[e.eliminate.page] || []).push(e.eliminate.class);
      delete st.probes[e.eliminate.page];
    }
    if (e.probe) st.probes[e.probe.page] = e.probe.class;
    if (e.anchor) {
      st.anchor = e.anchor;
      st.probes = {};
    }
    if (e.jobStart) st.jobs[e.jobStart.page + ":" + e.jobStart.jobtype] = { class: e.jobStart.class };
    if (e.jobEnd) delete st.jobs[e.jobEnd.page + ":" + e.jobEnd.jobtype];
    if (e.mode) {
      if (e.mode === "COMMITTED" && st.mode === "DEFERRAL") st.deferFrom = null;
      st.mode = e.mode;
    }
    if (e.deferFrom != null) st.deferFrom = e.deferFrom;
    if (e.deferRegionDone) st.deferRegions.push(e.deferRegionDone);
    if (e.parent) st.parent = e.parent;
    if (e.cursor != null) st.cursor = e.cursor;
    if (e.workers !== undefined) st.workers = e.workers;
    return st;
  }

  /** Replay state up to and including action index `idx` (-1 = initial). */
  function stateAt(model, idx, cache) {
    let st, start;
    if (cache && cache.idx != null && cache.idx <= idx) {
      st = cache.state;
      start = cache.idx + 1;
    } else {
      st = initialState(model.meta);
      start = 0;
    }
    for (let i = start; i <= idx && i < model.actions.length; i++) {
      applyEffects(st, model.actions[i]);
    }
    if (cache) {
      cache.idx = Math.min(idx, model.actions.length - 1);
      cache.state = st;
    }
    return st;
  }

  const api = { buildModel, stateAt, initialState, applyEffects, parseSoftSort };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  global.LogModel = api;
})(typeof window !== "undefined" ? window : globalThis);
