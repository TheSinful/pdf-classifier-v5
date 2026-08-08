from .user_func import UserFunc, FuncSyntax
from pathlib import Path
from dataclasses import dataclass
from typing import Iterator, Optional
import re
import logging

logger = logging.getLogger(__name__)

# classify/extract are virtuals on pdf_classifier_lib's Object, so both the name
# and the signature are fixed by the library rather than by the user's schema.
CLASSIFY_SYNTAX = FuncSyntax("ClassificationResult", ["Attached&"], "classify")
EXTRACT_SYNTAX = FuncSyntax("ExtractionResult", ["Attached&"], "extract")

# Comments and string/char literals are removed in a single pass so that a `//`
# inside a literal can't terminate a comment (and vice versa). Without this a
# doc comment mentioning classify() parses as a declaration.
_NOISE = re.compile(
    r'//[^\n]*'
    r'|/\*.*?\*/'
    r'|"(?:\\.|[^"\\])*"'
    r"|'(?:\\.|[^'\\])*'",
    re.DOTALL,
)

# Preprocessor lines are dropped whole (continuations included): an #if branch
# can open a brace that only closes in the other branch, which would desync the
# scope stack for the rest of the file.
_PREPROCESSOR = re.compile(r'^[ \t]*\#(?:[^\n\\]|\\.)*', re.MULTILINE | re.DOTALL)

# Matches the tail of the text preceding a `{`, to decide whether that brace
# opens a class scope. `enum class` is captured so it can be rejected - it opens
# a scope but not one that can hold member functions. The base clause is kept:
# an override can live on a base, and inheritance emits no declaration in the
# derived class for a regex to find.
_CLASS_HEAD = re.compile(
    r'(?P<enum>\benum\s+)?\b(?:class|struct|union)\s+(?P<name>\w+)'
    r'(?:\s+final)?(?:\s*:(?P<bases>[^{;]*))?\s*$'
)

# The library base. It declares classify/extract as pure virtuals and lives
# outside the project directory, so a lookup reaching it has ended without an
# implementation - a definite error rather than an unresolved base.
LIBRARY_BASE = "Object"

# Type qualifiers that stack in front of the type proper, so `const unsigned
# char*` reads as one type rather than as `const` followed by a name.
_QUALIFIERS = r'(?:(?:const|volatile|unsigned|signed|long|short|struct|enum)\s+)*'

# Anchored at the end of a statement, so leading noise ("public:", "virtual",
# "[[nodiscard]]") is skipped and the declaration itself is what gets captured.
# `ret` and `ptr` are split because the separator has to be real - without it
# `~Diagram()` parses as a function `m` returning `Diagra`. That also means
# constructors and destructors are skipped, which is correct: they have no
# return type, so there is nothing here to validate.
_FUNC = re.compile(
    r'(?P<ret>' + _QUALIFIERS + r'(?:\w+::)*\w+(?:\s*<[^<>]*>)?)'
    r'(?P<ptr>\s*[*&]+\s*|\s+)'
    r'(?P<name>\w+)\s*'
    r'\((?P<params>[^()]*)\)'
    r'(?:\s*(?:const|noexcept|override|final))*'
    r'(?:\s*=\s*(?:0|default|delete))?'
    r'\s*$'
)

# The separator before the parameter name is `\s*`, not `\s+`: `Attached &att`
# binds the ampersand to the name and is the same type as `Attached& att`. The
# type group is greedy, so an unnamed `Attached` stays whole rather than
# splitting into `Attache` + `d`.
_PARAM = re.compile(
    r'^\s*(?P<type>' + _QUALIFIERS + r'(?:\w+::)*\w+(?:\s*<[^<>]*>)?(?:\s*[*&]+)?)'
    r'(?:\s*(?P<name>\w+))?\s*(?:=\s*.+)?$',
    re.DOTALL,
)


def _normalize_type(t: str) -> str:
    """`Attached &`, `Attached&` and `Attached  &` all name the same type."""
    t = re.sub(r'\s+', ' ', t.strip())
    return re.sub(r'\s*([*&]+)\s*', r'\1', t)


def _normalize_class(name: str) -> str:
    """DataTable, data_table and datatable all name the object "datatable"."""
    return name.replace("_", "").lower()


def _parse_bases(clause: str) -> list[str]:
    """`: public ColorObject, private detail::Mixin<int>` -> [ColorObject, Mixin]."""
    bases: list[str] = []
    for part in _split_params(clause):
        part = re.sub(r'\b(?:public|protected|private|virtual)\b', ' ', part)
        part = re.sub(r'<[^<>]*>', ' ', part)
        names = re.findall(r'[\w:]+', part)
        if names:
            bases.append(names[-1].split("::")[-1])
    return bases


def _split_params(params: str) -> list[str]:
    """Split on top-level commas only, so std::pair<int, int> stays one param."""
    parts: list[str] = []
    depth = 0
    current = ""
    for ch in params:
        if ch in "<([{":
            depth += 1
        elif ch in ">)]}":
            depth -= 1
        elif ch == "," and depth == 0:
            parts.append(current)
            current = ""
            continue
        current += ch
    if current.strip():
        parts.append(current)
    return parts


@dataclass
class ParsedFunc:
    name: str
    return_type: str
    parameters: list[dict[str, str]]
    file: str
    # None for a declaration at namespace scope; the class name for a member.
    class_name: Optional[str] = None

    @property
    def qualified_name(self) -> str:
        return f"{self.class_name}::{self.name}" if self.class_name else self.name

class UserFuncValidator:
    expected_classify_funcs: list[UserFunc]
    expected_extract_funcs: list[UserFunc]
    user_cmake_lists_path: Path
    expected_classify_syntax: FuncSyntax
    expected_extract_syntax: FuncSyntax

    def __init__(self, expected_classify_funcs: list[UserFunc],
                expected_extract_funcs: list[UserFunc],
                user_cmake_lists_path: Path,
                expected_classify_syntax: FuncSyntax = CLASSIFY_SYNTAX,
                expected_extract_syntax: FuncSyntax = EXTRACT_SYNTAX) -> None:
        self.expected_classify_funcs = expected_classify_funcs
        self.expected_extract_funcs = expected_extract_funcs
        self.expected_classify_syntax = expected_classify_syntax
        self.expected_extract_syntax = expected_extract_syntax
        self.user_cmake_lists_path = user_cmake_lists_path


    def validate(self) -> None:
        logger.info("Validating user functions in %s", self.user_cmake_lists_path.parent)
        funcs, hierarchy = self._scan_project()
        logger.debug("Found %d function declarations and %d classes in project headers",
                     len(funcs), len(hierarchy))

        missing_classify = self._unsatisfied(funcs, hierarchy, self.expected_classify_funcs, self.expected_classify_syntax)
        missing_extract = self._unsatisfied(funcs, hierarchy, self.expected_extract_funcs, self.expected_extract_syntax)

        if missing_classify:
            raise RuntimeError(
                "Missing or mismatched classify declaration(s) in the project headers:\n  "
                + "\n  ".join(missing_classify)
            )
        if missing_extract:
            raise RuntimeError(
                "Missing or mismatched extract declaration(s) in the project headers:\n  "
                + "\n  ".join(missing_extract)
            )
        logger.info("Function validation passed")

    def _unsatisfied(self, funcs: list[ParsedFunc], hierarchy: dict[str, list[str]],
                     expected_list: list[UserFunc], expected_syntax: FuncSyntax) -> list[str]:
        """Describe every expected object that no parsed declaration satisfies."""
        missing: list[str] = []
        for expected in self._dedupe(expected_list, expected_syntax):
            problem = self._problem_with(funcs, hierarchy, expected, expected_syntax)
            if problem is not None:
                missing.append(problem)
        return missing

    def _problem_with(self, funcs: list[ParsedFunc], hierarchy: dict[str, list[str]],
                      expected: UserFunc, syntax: FuncSyntax) -> Optional[str]:
        """None when this object's method is accounted for, else why it isn't."""
        if not syntax.method_name:
            if any(self._validate_func(f, [expected], syntax) for f in funcs):
                return None
            return self._describe(expected, syntax)

        owner = self._resolve_class(hierarchy, expected)
        if owner is None:
            return f"{self._describe(expected, syntax)}\n      no such class is declared in the project headers"

        # A derived class inherits the override without redeclaring it, so the
        # whole chain is eligible - `Section` is satisfied by `ColorObject`.
        chain, unresolved = self._ancestors(hierarchy, owner)
        if any(f.class_name in chain and self._validate_signature(f, syntax) for f in funcs):
            return None

        if unresolved:
            # A base we never saw may carry it. Can't prove a failure, so say so
            # and let the compiler have the last word.
            logger.warning("Could not confirm %s::%s - base class(es) %s are not declared in %s",
                           owner, syntax.method_name, ", ".join(sorted(unresolved)),
                           self.user_cmake_lists_path.parent)
            return None

        searched = " -> ".join(self._inheritance_order(hierarchy, owner))
        return f"{self._describe(expected, syntax)}\n      searched {searched}"

    def _resolve_class(self, hierarchy: dict[str, list[str]], expected: UserFunc) -> Optional[str]:
        """The concrete class implementing this object, as declared in a header."""
        if expected.cpp_class:
            return expected.cpp_class if expected.cpp_class in hierarchy else None

        target = _normalize_class(expected.for_class)
        return next((c for c in hierarchy if _normalize_class(c) == target), None)

    def _ancestors(self, hierarchy: dict[str, list[str]], cls: str) -> tuple[set[str], set[str]]:
        """(every class an override could live on, bases we couldn't find)."""
        chain: set[str] = set()
        unresolved: set[str] = set()
        queue = [cls]

        while queue:
            current = queue.pop()
            if current in chain:
                continue
            if current not in hierarchy:
                # Object is the library's own base: it declares the pure virtuals
                # and implements neither, so ending there is a real failure.
                if current != LIBRARY_BASE:
                    unresolved.add(current)
                continue
            chain.add(current)
            queue.extend(hierarchy[current])

        return chain, unresolved

    def _inheritance_order(self, hierarchy: dict[str, list[str]], cls: str) -> list[str]:
        """The chain in declaration order, for the error message."""
        order: list[str] = []
        queue = [cls]
        while queue:
            current = queue.pop(0)
            if current in order:
                continue
            order.append(current)
            queue.extend(hierarchy.get(current, []))
        return order

    def _dedupe(self, expected_list: list[UserFunc], expected_syntax: FuncSyntax) -> list[UserFunc]:
        """One expectation per object - two headers declaring the same class is
        still a single thing to look for."""
        seen: set[str] = set()
        unique: list[UserFunc] = []
        for expected in expected_list:
            key = self._expected_class(expected) if expected_syntax.method_name else expected.name
            if key in seen:
                continue
            seen.add(key)
            unique.append(expected)
        return unique

    def _expected_class(self, expected: UserFunc) -> str:
        return expected.cpp_class if expected.cpp_class else expected.for_class

    def _describe(self, expected: UserFunc, expected_syntax: FuncSyntax) -> str:
        params = ", ".join(expected_syntax.param_types)
        if not expected_syntax.method_name:
            return f"{expected_syntax.return_type} {expected.name}({params})"

        owner = expected.cpp_class if expected.cpp_class else f"<class named like '{expected.for_class}'>"
        return (f"{expected_syntax.return_type} {owner}::{expected_syntax.method_name}({params})"
                f"   [object '{expected.for_class}', declared in {expected.file_name}]")

    def _validate_classify_func(self, func: ParsedFunc) -> bool:
        return self._validate_func(func, self.expected_classify_funcs, self.expected_classify_syntax)

    def _validate_extract_func(self, func: ParsedFunc) -> bool:
        return self._validate_func(func, self.expected_extract_funcs, self.expected_extract_syntax)

    def _validate_func(self, func: ParsedFunc, expected_list: list[UserFunc],
                       expected_syntax: FuncSyntax) -> bool:
        """Whether this declaration *directly* satisfies one of the expectations.
        validate() additionally resolves through base classes; this does not."""
        for expected in expected_list:
            if not self._validate_func_name(func, expected, expected_syntax):
                continue
            if not self._validate_signature(func, expected_syntax, check_name=False):
                continue
            return True
        return False

    def _validate_signature(self, func: ParsedFunc, expected_syntax: FuncSyntax,
                            check_name: bool = True) -> bool:
        """Name, return type and parameters - everything except which class owns it."""
        if check_name and func.name != expected_syntax.method_name:
            return False
        if _normalize_type(func.return_type) != _normalize_type(expected_syntax.return_type):
            return False
        if len(func.parameters) != len(expected_syntax.param_types):
            return False
        return self._validate_func_param_types(expected_syntax.param_types, func.parameters)

    def _validate_func_name(self, func: ParsedFunc, expected: UserFunc,
                            expected_syntax: FuncSyntax) -> bool:
        if not expected_syntax.method_name:
            # Legacy free-function form: Result* classify_chapter(...).
            return func.class_name is None and func.name == expected.name

        # Member form: ClassificationResult Chapter::classify(...).
        if func.name != expected_syntax.method_name:
            return False
        return self._validate_func_class(func, expected)

    def _validate_func_class(self, func: ParsedFunc, expected: UserFunc) -> bool:
        if func.class_name is None:
            return False
        if expected.cpp_class:
            return func.class_name == expected.cpp_class
        return _normalize_class(func.class_name) == _normalize_class(expected.for_class)

    def _validate_func_param_types(self, expected_types: list[str], given_params: list[dict[str, str]]) -> bool:
        return all(
            _normalize_type(expected) == _normalize_type(param["type"])
            for expected, param in zip(expected_types, given_params)
        )

    def _get_available_functions(self) -> list[ParsedFunc]:
        return self._scan_project()[0]

    def _scan_project(self) -> tuple[list[ParsedFunc], dict[str, list[str]]]:
        """Every declaration in the project headers, plus {class: [base classes]}."""
        functions: list[ParsedFunc] = []
        hierarchy: dict[str, list[str]] = {}
        project_dir = self.user_cmake_lists_path.parent
        header_files = list(project_dir.glob('*.h*'))
        logger.debug("Scanning %d header file(s) in %s", len(header_files), project_dir)

        for header_file in header_files:
            logger.debug("Parsing header: %s", header_file)
            with open(header_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()

            statements, bases = self._scan(content)
            hierarchy.update(bases)

            for class_name, statement in statements:
                match = _FUNC.search(statement)
                if match is None:
                    continue

                parsed = ParsedFunc(
                    name=match.group("name"),
                    return_type=_normalize_type(match.group("ret") + match.group("ptr")),
                    parameters=self._parse_params(match.group("params")),
                    file=str(header_file.absolute()),
                    class_name=class_name,
                )
                logger.debug("  Parsed function: %s %s(%s)", parsed.return_type, parsed.qualified_name,
                             ", ".join(p['type'] for p in parsed.parameters))
                functions.append(parsed)

        return functions, hierarchy

    def _parse_params(self, params_str: str) -> list[dict[str, str]]:
        if not params_str.strip() or params_str.strip() == 'void':
            return []

        parameters: list[dict[str, str]] = []
        for param in _split_params(params_str):
            match = _PARAM.match(param.strip())
            if match:
                parameters.append({'name': match.group("name") or "", 'type': _normalize_type(match.group("type"))})
            else:
                # Keep the raw text so an unparseable param fails loudly against
                # the expected signature instead of silently matching.
                parameters.append({'name': "", 'type': param.strip()})
        return parameters

    def _iter_statements(self, content: str) -> Iterator[tuple[Optional[str], str]]:
        yield from self._scan(content)[0]

    def _scan(self, content: str) -> tuple[list[tuple[Optional[str], str]], dict[str, list[str]]]:
        """Walk the braces once, producing every `;`-terminated statement tagged
        with the class that encloses it, plus each class's base list."""
        text = _PREPROCESSOR.sub('', _NOISE.sub(' ', content))

        # One entry per open brace: the class it opens, or None for any other
        # scope (function body, namespace, enum, initializer).
        scope: list[Optional[str]] = []
        statements: list[tuple[Optional[str], str]] = []
        hierarchy: dict[str, list[str]] = {}
        pos = 0

        for delim in re.finditer(r'[{};]', text):
            chunk = text[pos:delim.start()]
            pos = delim.end()

            if delim.group() == '{':
                opened = _CLASS_HEAD.search(chunk)
                if opened is None or opened.group("enum"):
                    scope.append(None)
                    continue
                name = opened.group("name")
                scope.append(name)
                hierarchy[name] = _parse_bases(opened.group("bases") or "")
            elif delim.group() == '}':
                if scope:
                    scope.pop()
            else:
                statements.append(((scope[-1] if scope else None), chunk))

        return statements, hierarchy
