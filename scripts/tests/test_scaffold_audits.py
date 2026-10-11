#!/usr/bin/env python3
"""The shipped scaffold files: the usage and spelling audits, the comment
exemption of the layout audit, and the export's vocabulary round trip."""

import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
EXPORT_SCRIPT = ROOT / "scripts" / "export_student_repo.sh"
USAGE_AUDIT = ROOT / "scripts" / "audit_scaffold_usage.pl"
SPELLING_AUDIT = ROOT / "scripts" / "audit_scaffold_spellings.pl"
LAYOUT_AUDIT = ROOT / "scripts" / "audit_compiler_layout.pl"
ROUND_TRIP = ROOT / "scripts" / "scaffold_vocabulary_round_trip.cpp"


def shipped_files() -> list[str]:
    match = re.search(r"^dev_support_files=\(\n(?P<body>.*?)^\)$",
                      EXPORT_SCRIPT.read_text(), re.M | re.S)
    if not match:
        raise AssertionError("export_student_repo.sh has no dev_support_files")
    return [line.strip() for line in match.group("body").splitlines() if line.strip()]


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def run(*command: object) -> subprocess.CompletedProcess:
    return subprocess.run([str(part) for part in command], capture_output=True,
                          text=True, check=False)


class ScaffoldUsageAudit(unittest.TestCase):
    """A tree whose export ships one vocabulary header, the scaffold-only
    not-implemented header and the test runner."""

    def tree(self, root: Path, deps: dict[str, str], scaffold_includes: bool = True) -> None:
        write(root, "scripts/export_student_repo.sh",
              "dev_support_files=(\n"
              "  dev/src/vocabulary.h\n"
              "  dev/src/support/not_implemented.h\n"
              "  dev/src/support/testing/test_runner.cpp\n"
              ")\n")
        write(root, "dev/frontend_source_sets.mk",
              "FRONTEND_TEST_RUNNER_SOURCE_ID := support/testing/test_runner\n")
        for relative in ("dev/src/vocabulary.h", "dev/src/support/not_implemented.h",
                         "dev/src/support/testing/test_runner.cpp", "dev/src/model.cpp",
                         "dev/tool.cpp"):
            write(root, relative, "\n")
        write(root, "dev/tool-scaffold.cpp",
              '#include "support/not_implemented.h"\n' if scaffold_includes else "\n")
        for relative, text in deps.items():
            write(root, f"obj/dev/.d/{relative}", text)

    def audit(self, root: Path) -> subprocess.CompletedProcess:
        return run("perl", USAGE_AUDIT, "--root", root)

    def test_passes_when_the_build_uses_every_shipped_header(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, {
                "model.d": "../obj/dev/model.o: src/model.cpp src/vocabulary.h\n",
                "entry/tool.d": "../obj/dev/entry/tool.o: tool.cpp src/vocabulary.h\n",
                # A dependency file whose source was deleted is skipped.
                "old.d": "../obj/dev/old.o: src/old.cpp src/support/not_implemented.h\n",
            })
            result = self.audit(root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("1 shipped headers compiled into the implementation", result.stdout)
            self.assertIn("1 left over from deleted sources skipped", result.stdout)

    def test_fails_when_a_shipped_header_is_not_compiled_in(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, {"model.d": "../obj/dev/model.o: src/model.cpp\n"})
            result = self.audit(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("dev/src/vocabulary.h is used by no implementation object", result.stderr)

    def test_fails_when_the_implementation_uses_a_scaffold_only_header(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, {"model.d": "../obj/dev/model.o: src/model.cpp src/vocabulary.h "
                                        "src/support/not_implemented.h\n"})
            result = self.audit(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("not_implemented.h is scaffold-only", result.stderr)
            self.assertIn("model.d", result.stderr)

    def test_fails_when_no_scaffold_includes_the_scaffold_only_header(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, {"model.d": "../obj/dev/model.o: src/model.cpp src/vocabulary.h\n"},
                      scaffold_includes=False)
            result = self.audit(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("is included by no student scaffold", result.stderr)

    def test_fails_without_a_current_build(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, {"old.d": "../obj/dev/old.o: src/old.cpp src/vocabulary.h\n"})
            result = self.audit(root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("build the tools first", result.stderr)


class ScaffoldSpellingAudit(unittest.TestCase):
    def tree(self, root: Path, writer_literal: str) -> None:
        write(root, "dev/src/lowir_model.h",
              'inline const char * spelling(Keyword k)\n'
              '{ static const char * const kKeywords[2] = { "declare", "global" }; return kKeywords[k]; }\n')
        write(root, "dev/src/ir_symbol_model.h",
              'static const char * const kStorages[2] = { "", "readonly" };\n')
        write(root, "dev/src/mir_model.h", 'static const char * const kOps[1] = { "mov" };\n')
        write(root, "dev/src/native/mir/registers.h",
              'static const char * const kGprs[1] = { "rax" };\n')
        write(root, "dev/src/lowir/text_writer.cpp",
              f'void write() {{ text("{writer_literal}"); internal_error("not a word: zz"); }}\n')
        write(root, "dev/src/lowir/literals.cpp", "\n")
        write(root, "dev/src/native/mir/mir_writer.cpp",
              'void dump() { text("  mov "); text("frame "); text(" rax"); }\n')
        for relative in ("text_parser.cpp", "text_parser_body.cpp", "text_lexer.cpp"):
            write(root, f"dev/src/lowir/{relative}", 'bool f() { return at_word("readonly"); }\n')

    def test_passes_when_writer_words_are_vocabulary_spellings(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, "declare global ")
            result = run("perl", SPELLING_AUDIT, "--root", root)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_fails_on_a_misspelled_writer_word(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.tree(root, "declare globl ")
            result = run("perl", SPELLING_AUDIT, "--root", root)
            self.assertEqual(result.returncode, 1)
            self.assertIn("LowIR writer: word 'globl'", result.stderr)

    def test_passes_on_this_tree(self):
        result = run("perl", SPELLING_AUDIT)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class LayoutAuditScaffoldComments(unittest.TestCase):
    def test_shipped_scaffold_comments_may_cite_handouts(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write(root, "scripts/architecture/compiler-layout-legacy.tsv", "kind\tpattern\tdestination\n")
            write(root, "scripts/architecture/compiler-pa-contract-allowlist.tsv",
                  "kind\tpath\ttoken\treason\n")
            write(root, "scripts/export_student_repo.sh", "dev_support_files=(\n  dev/src/shipped.h\n)\n")
            shutil.copy2(LAYOUT_AUDIT, root / "scripts" / LAYOUT_AUDIT.name)
            write(root, "dev/src/shipped.h", "// The format of pa8/lowir.md (PA8).\nint shipped;\n")
            write(root, "dev/src/model.cpp", "int model;\n")
            result = run("perl", root / "scripts" / LAYOUT_AUDIT.name)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("2 handout citations in shipped scaffold comments", result.stdout)

            # The implementation's comments still need an allowlist row, and
            # a shipped file's code is audited as usual.
            write(root, "dev/src/model.cpp", "// See PA8.\nint model;\n")
            write(root, "dev/src/shipped.h", "// PA8.\nconst char * shipped = \"pa8_key\";\n")
            result = run("perl", root / "scripts" / LAYOUT_AUDIT.name)
            self.assertEqual(result.returncode, 1)
            self.assertIn("dev/src/model.cpp:1: unclassified comment PA reference 'PA8'", result.stdout)
            self.assertIn("dev/src/shipped.h:2: unclassified diagnostic PA reference 'pa8_key'",
                          result.stdout)
            self.assertNotIn("dev/src/shipped.h:1:", result.stdout)


class ScaffoldVocabularyRoundTrip(unittest.TestCase):
    def test_round_trip_includes_every_shipped_header(self):
        # The export compiles the round trip against the exported headers;
        # a newly shipped header must join it.
        text = ROUND_TRIP.read_text()
        for path in shipped_files():
            if path.endswith(".h"):
                include = path.removeprefix("dev/src/")
                self.assertIn(f'#include "{include}"', text, path)
        script = EXPORT_SCRIPT.read_text()
        self.assertIn("scripts/scaffold_vocabulary_round_trip.cpp", script)
        self.assertRegex(script, r"(?m)^check_exported_scaffold_vocabulary$")
        self.assertRegex(script, r"(?m)^check_exported_tool_scaffolds$")

    def test_round_trip_passes_on_the_shipped_headers(self):
        compiler = os.environ.get("CXX", "g++")
        if shutil.which(compiler) is None:
            self.skipTest(f"{compiler} is not installed")
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / "round_trip"
            command = [compiler, "-std=gnu++11", "-Wall", "-Wextra", "-Werror",
                       *os.environ.get("CPPGM_STDLIB_FLAGS", "").split(),
                       "-I", str(ROOT / "dev" / "src"), "-o", str(binary), str(ROUND_TRIP)]
            built = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(built.returncode, 0, built.stderr)
            ran = run(binary)
            self.assertEqual(ran.returncode, 0, ran.stderr)


if __name__ == "__main__":
    unittest.main()
