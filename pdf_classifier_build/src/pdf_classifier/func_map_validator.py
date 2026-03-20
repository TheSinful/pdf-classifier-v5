from .user_func import UserFunc, FuncSyntax
from pathlib import Path
from dataclasses import dataclass
import re
import logging

logger = logging.getLogger(__name__)

@dataclass
class ParsedFunc:
    name: str
    return_type: str
    parameters: list[dict[str, str]]
    file: str

class UserFuncValidator:
    expected_classify_funcs: list[UserFunc]
    expected_extract_funcs: list[UserFunc]
    user_cmake_lists_path: Path
    expected_classify_syntax: FuncSyntax
    expected_extract_syntax: FuncSyntax

    def __init__(self, expected_classify_funcs: list[UserFunc],
                expected_extract_funcs: list[UserFunc],
                user_cmake_lists_path: Path,
                expected_classify_syntax: FuncSyntax = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*"]),
                expected_extract_syntax: FuncSyntax = FuncSyntax("Result*", ["uint32_t", "fz_context*", "fz_document*", "void*"])) -> None:
        self.expected_classify_funcs = expected_classify_funcs
        self.expected_extract_funcs = expected_extract_funcs
        self.expected_classify_syntax = expected_classify_syntax
        self.expected_extract_syntax = expected_extract_syntax
        self.user_cmake_lists_path = user_cmake_lists_path


    def validate(self) -> None:
        logger.info("Validating user functions in %s", self.user_cmake_lists_path.parent)
        funcs = self._get_available_functions()
        logger.debug("Found %d function declarations in project headers", len(funcs))

        classify_found = sum(1 for f in funcs if self._validate_classify_func(f))
        extract_found = sum(1 for f in funcs if self._validate_extract_func(f))

        expected_classify_count = len({f.name for f in self.expected_classify_funcs})
        expected_extract_count = len({f.name for f in self.expected_extract_funcs})

        logger.debug("Classify: found %d / expected %d", classify_found, expected_classify_count)
        logger.debug("Extract:  found %d / expected %d", extract_found, expected_extract_count)

        if classify_found != expected_classify_count:
            raise RuntimeError(
                f"Expected {expected_classify_count} classify functions, but found {classify_found}"
            )
        if extract_found != expected_extract_count:
            raise RuntimeError(
                f"Expected {expected_extract_count} extract functions, but found {extract_found}"
            )
        logger.info("Function validation passed")

    def _validate_classify_func(self, func: ParsedFunc) -> bool:
        return self._validate_func(func, self.expected_classify_funcs, self.expected_classify_syntax)

    def _validate_extract_func(self, func: ParsedFunc) -> bool:
        return self._validate_func(func, self.expected_extract_funcs, self.expected_extract_syntax)

    def _validate_func(self, func: ParsedFunc, expected_list: list[UserFunc],
                       expected_syntax: FuncSyntax) -> bool:
        for expected in expected_list:
            if func.name != expected.name:
                continue
            if func.return_type != expected_syntax.return_type:
                continue
            if len(func.parameters) != len(expected_syntax.param_types):
                continue
            if not self._validate_func_param_types(expected_syntax.param_types, func.parameters):
                continue
            return True
        return False

    def _validate_func_param_types(self, expected_types: list[str], given_params: list[dict[str, str]]) -> bool:
        return all(
            expected == param["type"]
            for expected, param in zip(expected_types, given_params)
        )

    def _get_available_functions(self) -> list[ParsedFunc]:
        func_pattern = r'((?:\w+::)*\w+\s*[\*&]?)\s+(\w+)\s*\((.*?)\)\s*;'
        param_pattern = r'((?:const\s+)?(?:\w+::)*\w+(?:\s*\*|\s*&)?)\s+(\w+)'

        functions: list[ParsedFunc] = []
        project_dir = self.user_cmake_lists_path.parent
        header_files = list(project_dir.glob('*.h*'))
        logger.debug("Scanning %d header file(s) in %s", len(header_files), project_dir)

        for header_file in header_files:
            logger.debug("Parsing header: %s", header_file)
            with open(header_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()

            for match in re.finditer(func_pattern, content):
                return_type, func_name, params_str = match.groups()

                parameters: list[dict[str, str]] = []
                if params_str.strip() and params_str.strip() != 'void':
                    for param in params_str.split(','):
                        param_match = re.search(param_pattern, param.strip())
                        if param_match:
                            param_type, param_name = param_match.groups()
                            parameters.append({'name': param_name, 'type': param_type.strip()})

                logger.debug("  Parsed function: %s %s(%s)", return_type.strip(), func_name,
                             ", ".join(p['type'] for p in parameters))
                functions.append(ParsedFunc(
                    name=func_name,
                    return_type=return_type.strip(),
                    parameters=parameters,
                    file=str(header_file.absolute()),
                ))

        return functions
