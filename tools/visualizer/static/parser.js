/**
 * parser.js — parses the classifier's `tracing-tree` (HierarchicalLayer) log output
 * into a tree of span/event entries.
 *
 * Format assumptions (verified against tracing-tree 0.4.1 with
 * `HierarchicalLayer::new(4)`, `indent_lines = false`, no timestamps, no targets):
 *
 *   span line:   " " + "    "*depth + name + " " + "k=v, k2=v2"      (fields may be empty)
 *   event line:  " " + "    "*depth + " " + LEVEL + " " + message[, k=v]*
 *                (the extra space before LEVEL is the empty-timestamp separator;
 *                 root events outside any span have a single leading space)
 *   span close:  prints nothing (default config)
 *   result line: "N -> class"   (the binary's final stdout, if merged into the log)
 *
 * Continuation lines of multi-line messages are indented +2 relative to the first
 * line and don't match any pattern — they get appended to the previous entry.
 *
 * Exposed as a plain global (`LogParser`) so it works from file:// and in node
 * (`require`) alike.
 */
(function (global) {
  "use strict";

  const ANSI_RE = /\x1b\[[0-9;]*m/g;
  const LEVELS = { TRACE: 1, DEBUG: 1, INFO: 1, WARN: 1, ERROR: 1 };
  const RESULT_RE = /^(\d+)\s*->\s*([\w-]+)\s*$/;
  const EVENT_RE = /^( +)(TRACE|DEBUG|INFO|WARN|ERROR)(?: (.*))?$/;
  const SPAN_RE = /^( +)([A-Za-z_][A-Za-z0-9_]*)(?: (.*))?$/;

  /** Split on top-level ", " — respecting (), [], {} nesting and "..." strings. */
  function splitTopLevel(s) {
    const parts = [];
    let cur = "";
    let depth = 0;
    let inStr = false;
    for (let i = 0; i < s.length; i++) {
      const c = s[i];
      if (inStr) {
        cur += c;
        if (c === "\\") {
          cur += s[++i] || "";
        } else if (c === '"') {
          inStr = false;
        }
        continue;
      }
      if (c === '"') {
        inStr = true;
        cur += c;
      } else if (c === "(" || c === "[" || c === "{" || c === "<") {
        depth++;
        cur += c;
      } else if (c === ")" || c === "]" || c === "}" || c === ">") {
        depth = Math.max(0, depth - 1);
        cur += c;
      } else if (c === "," && depth === 0) {
        parts.push(cur.trim());
        cur = "";
      } else {
        cur += c;
      }
    }
    if (cur.trim().length) parts.push(cur.trim());
    return parts;
  }

  const FIELD_RE = /^([A-Za-z_][A-Za-z0-9_.#]*)=(.*)$/s;

  /**
   * Parse "msg fragment, k=v, k2=v2" into { message, fields }.
   * Fragments without a top-level `k=` are treated as message text.
   */
  function parseContent(s) {
    const fields = {};
    const msgParts = [];
    for (const part of splitTopLevel(s || "")) {
      const m = FIELD_RE.exec(part);
      if (m) fields[m[1]] = m[2];
      else if (part.length) msgParts.push(part);
    }
    return { message: msgParts.join(", "), fields };
  }

  /** Does this content string look like a pure `k=v, ...` field list (or empty)? */
  function looksLikeFieldList(s) {
    if (!s || !s.trim().length) return true;
    const parts = splitTopLevel(s);
    return parts.length > 0 && FIELD_RE.test(parts[0]);
  }

  /**
   * Parse the whole log text.
   * Returns { root, all, results, lines } where:
   *   root    — synthetic root span whose children are the top-level entries
   *   all     — every entry in document order
   *   results — [{page, class, line}] from final-structure stdout lines
   *   lines   — the (ANSI-stripped) source lines, for the raw-log pane
   */
  function parseLog(text) {
    const lines = text.replace(ANSI_RE, "").split(/\r?\n/);
    const root = { kind: "span", name: "$root", fields: {}, depth: -1, line: -1, children: [], parent: null };
    const all = [];
    const results = [];
    const stack = [root];
    let last = null; // last entry, for continuation lines

    const top = () => stack[stack.length - 1];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (!line.trim().length) continue;

      const res = RESULT_RE.exec(line);
      if (res) {
        results.push({ page: parseInt(res[1], 10), class: res[2].toUpperCase(), line: i });
        continue;
      }

      const ev = EVENT_RE.exec(line);
      if (ev) {
        const leading = ev[1].length;
        const depth = Math.max(0, Math.round((leading - 2) / 4));
        while (top().depth >= 0 && top().depth > depth) stack.pop();
        const { message, fields } = parseContent(ev[3] || "");
        const entry = {
          kind: "event",
          level: ev[2],
          message,
          fields,
          depth,
          line: i,
          lineEnd: i,
          children: [],
          parent: top(),
        };
        top().children.push(entry);
        all.push(entry);
        last = entry;
        continue;
      }

      const sp = SPAN_RE.exec(line);
      if (sp && looksLikeFieldList(sp[3])) {
        const leading = sp[1].length;
        const depth = Math.max(0, Math.round((leading - 1) / 4));
        while (top().depth >= 0 && top().depth >= depth) stack.pop();
        const { fields } = parseContent(sp[3] || "");
        const entry = {
          kind: "span",
          name: sp[2],
          fields,
          depth,
          line: i,
          lineEnd: i,
          children: [],
          parent: top(),
        };
        top().children.push(entry);
        all.push(entry);
        stack.push(entry);
        last = entry;
        continue;
      }

      // Continuation of a multi-line message (or something we don't recognize).
      if (last) {
        last.message = (last.message ? last.message + "\n" : "") + line.trim();
        last.lineEnd = i;
      } else {
        const entry = {
          kind: "raw", message: line.trim(), fields: {}, depth: 0,
          line: i, lineEnd: i, children: [], parent: root,
        };
        root.children.push(entry);
        all.push(entry);
        last = entry;
      }
    }

    return { root, all, results, lines };
  }

  // ---- small value helpers shared with the model ----

  /** "Page(44)" | "44" → 44 ; anything else → null */
  function pageNum(v) {
    if (v == null) return null;
    const m = /^(?:Page\()?(\d+)\)?$/.exec(String(v).trim());
    return m ? parseInt(m[1], 10) : null;
  }

  /** normalize a class value ("chapter" | "CHAPTER") → "CHAPTER" */
  function className(v) {
    if (v == null) return null;
    return String(v).trim().toUpperCase();
  }

  /** parse `res=` values: Ok(...) / Err("why") / Fail(FailUserResult { err: "why" }) */
  function parseResValue(v) {
    if (v == null) return null;
    const s = String(v).trim();
    if (s.startsWith("Ok(")) return { ok: true, reason: null };
    let m = /^Err\("((?:[^"\\]|\\.)*)"\)$/s.exec(s);
    if (m) return { ok: false, reason: m[1] };
    m = /err:\s*"((?:[^"\\]|\\.)*)"/s.exec(s);
    if (m) return { ok: false, reason: m[1] };
    if (s.startsWith("Fail(") || s.startsWith("Err(")) return { ok: false, reason: s };
    return { ok: true, reason: null };
  }

  const api = { parseLog, splitTopLevel, parseContent, pageNum, className, parseResValue };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  global.LogParser = api;
})(typeof window !== "undefined" ? window : globalThis);
