#!/usr/bin/env python3
"""Generate FzBinding tags and FzOwned aliases for MuPDF types.

Reads the MuPDF headers and codegen/fz_tags.toml, writes:
  generated/fz_bindings.hpp  FzBinding, fz_binding_of<T> and one specialisation per tagged type.
                             Depends only on MuPDF, so wrappers.hpp can include it.
  generated/fz_wrappers.hpp  FzWrap<T, Drop> and one alias per drop function. Includes wrappers.hpp.

Usage (from anywhere):
  python pdf_classifier_lib/codegen/gen_fz_wrappers.py [--mupdf-src DIR] [--check] [--strict]

  --mupdf-src  MuPDF source root (default: examples/build/mupdf_build/_deps/mupdf-src)
  --check      Don't write; exit 1 if the generated files are out of date.
  --strict     Treat warnings (version mismatch, evidence drift, unclassified drops) as errors.

After a MuPDF bump: run it, read every WARNING, fix fz_tags.toml (re-verify evidence for each
drifted entry, classify each new drop function), rerun until it's clean, commit both files.
Warnings never fail the build: a version mismatch also shows up as a #pragma message at compile time.

Requires Python 3.11+ (tomllib).
"""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

if sys.version_info < (3, 11):
    sys.exit("gen_fz_wrappers.py needs Python 3.11+ (for tomllib). On this machine try `python` instead of `python3`.")

import tomllib  # noqa: E402

HERE = Path(__file__).resolve().parent
LIB = HERE.parent
REPO = LIB.parent
DEFAULT_MUPDF = REPO / "examples" / "build" / "mupdf_build" / "_deps" / "mupdf-src"
TAGS = HERE / "fz_tags.toml"
OUT_DIR = LIB / "generated"
TAGS_REL = "pdf_classifier_lib/codegen/fz_tags.toml"
SCRIPT_REL = "pdf_classifier_lib/codegen/gen_fz_wrappers.py"
BINDINGS = ("Context", "Document")

DROP_RE = re.compile(
    r"^[ \t]*(?P<ret>[A-Za-z_][\w \t\*]*?)[ \t]*\b(?P<name>(?:fz|pdf)_drop_\w+)\s*\((?P<args>[^;{)]*(?:\([^)]*\)[^;{)]*)*)\)\s*;",
    re.M,
)
STD_ARGS_RE = re.compile(r"^fz_context\s*\*\s*\w*\s*,\s*(?P<const>const\s+)?(?P<type>\w+)\s*\*\s*\w*$")
VERSION_RE = re.compile(r'#define\s+FZ_VERSION\s+"(\d+)\.(\d+)\.(\d+)"')

warnings: list[str] = []


def warn(msg: str) -> None:
    warnings.append(msg)
    print(f"WARNING: {msg}", file=sys.stderr)


def fail(msg: str) -> None:
    sys.exit(f"ERROR: {msg}")


def strip_comments(text: str) -> str:
    blank = lambda m: re.sub(r"[^\n]", " ", m.group(0))  # noqa: E731 - keep line numbers
    text = re.sub(r"/\*.*?\*/", blank, text, flags=re.S)
    return re.sub(r"//[^\n]*", blank, text)


@dataclass
class Drop:
    name: str
    type: str | None  # None = non-standard shape
    const: bool
    where: str
    signature: str


def scan_drops(mupdf: Path) -> list[Drop]:
    inc = mupdf / "include"
    drops: dict[str, Drop] = {}
    for header in sorted(inc.rglob("*.h")):
        text = strip_comments(header.read_text(encoding="utf-8", errors="replace"))
        for m in DROP_RE.finditer(text):
            name = m.group("name")
            if name in drops:
                continue
            ret = " ".join(m.group("ret").split())
            args = " ".join(m.group("args").split())
            sm = STD_ARGS_RE.match(args)
            std = ret == "void" and sm is not None
            line = text.count("\n", 0, m.start("name")) + 1
            drops[name] = Drop(
                name=name,
                type=sm.group("type") if std else None,
                const=bool(std and sm.group("const")),
                where=f"{header.relative_to(mupdf).as_posix()}:{line}",
                signature=f"{ret} {name}({args})",
            )
    return sorted(drops.values(), key=lambda d: d.name)


def mupdf_version(mupdf: Path) -> tuple[int, int, int]:
    vh = mupdf / "include" / "mupdf" / "fitz" / "version.h"
    m = VERSION_RE.search(vh.read_text(encoding="utf-8", errors="replace"))
    if not m:
        fail(f"can't read FZ_VERSION from {vh}")
    return tuple(int(x) for x in m.groups())  # type: ignore[return-value]


def load_tags() -> dict:
    try:
        with TAGS.open("rb") as f:
            tags = tomllib.load(f)
    except tomllib.TOMLDecodeError as e:
        fail(f"{TAGS_REL} is not valid TOML: {e}")
    if not re.fullmatch(r"\d+\.\d+\.\d+", str(tags.get("mupdf_version", ""))):
        fail(f"{TAGS_REL}: mupdf_version must be \"X.Y.Z\"")
    for sect in ("types", "uncertain"):
        for t, e in tags.get(sect, {}).items():
            where = f"{TAGS_REL} [{sect}.{t}]"
            if sect == "types" and e.get("tag") not in BINDINGS:
                fail(f"{where}: tag must be one of {BINDINGS}")
            if not str(e.get("reason", "")).strip():
                fail(f"{where}: reason is required")
            ev = e.get("evidence")
            if not isinstance(ev, list) or not ev or not all(isinstance(x, dict) and "at" in x and "contains" in x for x in ev):
                fail(f"{where}: evidence must be a non-empty list of {{ at = \"file:line\", contains = \"...\" }}")
    both = set(tags.get("types", {})) & set(tags.get("uncertain", {}))
    if both:
        fail(f"{TAGS_REL}: types listed as both tagged and uncertain: {sorted(both)}")
    for name, e in tags.get("skip", {}).items():
        if not str(e.get("reason", "")).strip():
            fail(f"{TAGS_REL} [skip.{name}]: reason is required")
    return tags


def check_evidence(mupdf: Path, tags: dict) -> None:
    cache: dict[str, list[str] | None] = {}
    for sect in ("types", "uncertain"):
        for t, e in tags.get(sect, {}).items():
            for ev in e["evidence"]:
                f, _, n = ev["at"].rpartition(":")
                if f not in cache:
                    p = mupdf / f
                    cache[f] = p.read_text(encoding="utf-8", errors="replace").splitlines() if p.is_file() else None
                lines = cache[f]
                if lines is None:
                    warn(f"[{sect}.{t}] evidence file missing: {f}")
                elif not n.isdigit() or not 1 <= int(n) <= len(lines):
                    warn(f"[{sect}.{t}] evidence line out of range: {ev['at']}")
                elif ev["contains"] not in lines[int(n) - 1]:
                    hits = [i + 1 for i, l in enumerate(lines) if ev["contains"] in l]
                    moved = f" (text now at line {', '.join(map(str, hits[:3]))})" if hits else " (text not found in file)"
                    warn(f"[{sect}.{t}] evidence drifted: {ev['at']} no longer contains {ev['contains']!r}{moved}")


def alias_name(drop: str, override: str | None) -> str:
    if override:
        return override
    prefix, _, rest = drop.partition("_drop_")
    return prefix.capitalize() + "".join(w.capitalize() for w in rest.split("_"))


def c_comment(s: str) -> str:
    return s.replace("*/", "* /")


def render_bindings(ver: tuple[int, int, int], tags: dict, drops: list[Drop]) -> str:
    pinned = tuple(int(x) for x in tags["mupdf_version"].split("."))
    tagged = tags.get("types", {})
    uncertain = tags.get("uncertain", {})
    untagged_known = sorted({d.type for d in drops if d.type and d.type not in tagged and d.name not in tags.get("skip", {})})
    out = [
        f"// GENERATED by {SCRIPT_REL} from {TAGS_REL}. Do not edit by hand.",
        f"// Tags checked against MuPDF {tags['mupdf_version']}.",
        "//",
        "// FzBinding says whether a MuPDF object may outlive a classify call:",
        "//   Context   owned only by the shared fz_context family; may move to any worker.",
        "//   Document  holds (directly or transitively) a worker's fz_document / fz_page / pdf_document.",
        "#pragma once",
        "",
        "#include <mupdf/fitz.h>",
        "#include <mupdf/pdf.h>",
        "",
        f"#if FZ_VERSION_MAJOR != {pinned[0]} || FZ_VERSION_MINOR != {pinned[1]} || FZ_VERSION_PATCH != {pinned[2]}",
        f'#pragma message("fz_bindings.hpp: FzBinding tags were checked against MuPDF {tags["mupdf_version"]}, but this build uses MuPDF " FZ_VERSION ". Re-verify the tags: python {SCRIPT_REL}")',
        "#endif",
        "",
        "enum class FzBinding { Context, Document };",
        "",
        "namespace fz_binding_detail {",
        "template <class> inline constexpr bool untagged = false;",
        "}",
        "",
        "// Primary template: any type without a specialisation below fails here.",
        "template <class T> struct fz_binding_of {",
        "  static_assert(fz_binding_detail::untagged<T>,",
        f'                "This MuPDF type has no FzBinding tag. Add [types.<type>] to {TAGS_REL} "',
        '                "with tag = \\"Context\\" or \\"Document\\", a reason and file:line evidence, "',
        f'                "then run: python {SCRIPT_REL}. The type is named in the instantiation note below.");',
        "};",
        "",
        "template <class T>",
        "inline constexpr FzBinding fz_binding_v = fz_binding_of<T>::value;",
        "",
    ]
    for tag in BINDINGS:
        entries = sorted((t, e) for t, e in tagged.items() if e["tag"] == tag)
        out.append(f"// ---- {tag} ({len(entries)}) " + "-" * 60)
        for t, e in entries:
            out.append(f"// {c_comment(e['reason'])}")
            out.append(f"// evidence: {e['evidence'][0]['at']}" + (f" (+{len(e['evidence']) - 1} more in fz_tags.toml)" if len(e["evidence"]) > 1 else ""))
            out.append(f"template <> struct fz_binding_of<{t}> {{ static constexpr FzBinding value = FzBinding::{tag}; }};")
        out.append("")
    if untagged_known:
        out.append(f"// ---- Untagged ({len(untagged_known)}): using these fails to compile until they're tagged " + "-" * 10)
        for t in untagged_known:
            why = uncertain[t]["reason"] if t in uncertain else "not in fz_tags.toml (new since the last MuPDF bump?)"
            out.append(f"// {t}: {c_comment(why)}")
        out.append("")
    return "\n".join(out)


def render_wrappers(tags: dict, drops: list[Drop]) -> str:
    tagged = tags.get("types", {})
    skip = tags.get("skip", {})
    std = [d for d in drops if d.type and d.name not in skip]
    out = [
        f"// GENERATED by {SCRIPT_REL} from {TAGS_REL}. Do not edit by hand.",
        "#pragma once",
        "",
        '#include "fz_bindings.hpp"',
        '#include "../wrappers.hpp"',
        "",
        "// Use this for hand-written wrappers too: it fails to compile if T has no FzBinding tag.",
        "template <class T, void (*Drop)(fz_context *, T *)>",
        "using FzWrap = FzOwned<T, Drop, fz_binding_v<T>>;",
        "",
        "// Stand-in alias for untagged types: naming it is fine, using it fails with the reason.",
        "template <class T> struct FzUntagged {",
        "  static_assert(fz_binding_detail::untagged<T>,",
        f'                "This MuPDF type is untagged (uncertain, or new after a MuPDF bump) so it has no wrapper yet. "',
        f'                "See its entry in {TAGS_REL}, decide Context or Document, then run: python {SCRIPT_REL}");',
        "};",
        "",
        "// How Kept<T> is meant to check its argument (Kept<T> itself is hand-written, not generated):",
        "//   template <class T> class Kept {",
        "//     static_assert(fz_binding_v<typename T::element_type> == FzBinding::Context,",
        "//                   \"Kept<T> only holds Context-bound MuPDF objects; Document-bound ones must not outlive classify\");",
        "//     ...",
        "//   };",
        "",
    ]
    const_drops = [d for d in std if d.const]
    if const_drops:
        out.append("// MuPDF drops these through a const pointer; FzOwned needs Drop(fz_context*, T*).")
        out.append("namespace fz_drop_shim {")
        for d in const_drops:
            out.append(f"inline void {d.name}(fz_context *ctx, {d.type} *p) {{ ::{d.name}(ctx, p); }}")
        out.append("}")
        out.append("")
    rows_t, rows_u = [], []
    for d in std:
        e = tagged.get(d.type)
        # an `alias` override names the type's own drop (fz_drop_stext_page), not extra ones (pdf_drop_widget)
        prefix, _, suffix = d.type.partition("_")
        own_drop = d.name == f"{prefix}_drop_{suffix}"
        name = alias_name(d.name, e.get("alias") if e and own_drop else None)
        fn = f"fz_drop_shim::{d.name}" if d.const else d.name
        if e:
            rows_t.append(f"using {name} = FzOwned<{d.type}, {fn}, fz_binding_v<{d.type}>>; // {e['tag']}")
        else:
            rows_u.append(f"using {name} = FzUntagged<{d.type}>;")
    out.append(f"// ---- Tagged ({len(rows_t)}) " + "-" * 60)
    out.extend(rows_t)
    out.append("")
    if rows_u:
        out.append(f"// ---- Untagged ({len(rows_u)}) " + "-" * 60)
        out.extend(rows_u)
        out.append("")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--mupdf-src", type=Path, default=DEFAULT_MUPDF)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--strict", action="store_true")
    a = ap.parse_args()

    mupdf = a.mupdf_src.resolve()
    if not (mupdf / "include" / "mupdf" / "fitz.h").is_file():
        fail(f"not a MuPDF source root: {mupdf} (pass --mupdf-src; it's fetched by the examples CMake build)")

    tags = load_tags()
    ver = mupdf_version(mupdf)
    if ".".join(map(str, ver)) != tags["mupdf_version"]:
        warn(f"MuPDF is {'.'.join(map(str, ver))} but {TAGS_REL} was checked against {tags['mupdf_version']}. "
             f"Re-verify the evidence, then set mupdf_version.")

    drops = scan_drops(mupdf)
    std = [d for d in drops if d.type]
    tagged, uncertain, skip = tags.get("types", {}), tags.get("uncertain", {}), tags.get("skip", {})
    for d in std:
        if d.name not in skip and d.type not in tagged and d.type not in uncertain:
            warn(f"unclassified drop function {d.signature} at {d.where}: add [types.{d.type}], "
                 f"[uncertain.{d.type}] or [skip.{d.name}] to {TAGS_REL}")
    std_names, std_types = {d.name for d in std}, {d.type for d in std}
    for name in skip:
        if name not in std_names:
            warn(f"[skip.{name}] no longer matches a standard-shape drop function")
    for sect, entries in (("types", tagged), ("uncertain", uncertain)):
        for t in entries:
            if t not in std_types:
                warn(f"[{sect}.{t}] no drop function takes {t}* any more")
    check_evidence(mupdf, tags)

    files = {
        OUT_DIR / "fz_bindings.hpp": render_bindings(ver, tags, drops),
        OUT_DIR / "fz_wrappers.hpp": render_wrappers(tags, drops),
    }
    stale = [p for p, txt in files.items() if not p.is_file() or p.read_text(encoding="utf-8") != txt + "\n"]
    if a.check:
        for p in stale:
            print(f"out of date: {p.relative_to(REPO).as_posix()}", file=sys.stderr)
    else:
        OUT_DIR.mkdir(exist_ok=True)
        for p, txt in files.items():
            p.write_text(txt + "\n", encoding="utf-8", newline="\n")

    n_ctx = sum(e["tag"] == "Context" for e in tagged.values())
    nonstd = [d for d in drops if not d.type]
    print(f"MuPDF {'.'.join(map(str, ver))}: {len(drops)} drop functions found by the scanner "
          f"({len(std)} standard shape, {len(nonstd)} other); {len(skip)} skipped.")
    print(f"Tags: {n_ctx} Context, {len(tagged) - n_ctx} Document, {len(uncertain)} uncertain.")
    print(("Checked" if a.check else "Wrote") + f" {OUT_DIR.relative_to(REPO).as_posix()}/fz_bindings.hpp, fz_wrappers.hpp"
          + (f" ({len(warnings)} warning(s))" if warnings else ""))
    if a.check and stale:
        return 1
    return 1 if (a.strict and warnings) else 0


if __name__ == "__main__":
    sys.exit(main())
