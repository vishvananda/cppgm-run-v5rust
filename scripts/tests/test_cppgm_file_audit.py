import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
AUDIT = ROOT / "scripts" / "cppgm_file_audit.pl"


class FileAuditFunctions(unittest.TestCase):
    def audit(self, member_body):
        # A short initializer-list constructor used to remain in the signature
        # buffer and turn the following entire class into a function named tail.
        source = """struct Step
{
    int tail;
    int temporary;
    int type;
    int destructor;
    int destination;
    Step(int previous, int action,
        int object_type, int binding,
        int address)
        : tail(previous), temporary(action), type(object_type),
          destructor(binding), destination(address) {}
};
template <class Derived>
class Prefix
{
public:
    Prefix() : root(0), active(false) {}
    void member()
    {
BODY
    }
METHODS
private:
    int root;
    bool active;
};
"""
        methods = "\n".join(
            f"    int value{i}() const\n    {{\n        return {i};\n    }}\n"
            for i in range(50)
        )
        source = source.replace("BODY", member_body).replace("METHODS", methods)
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            (root / "prefix.cpp").write_text(source)
            return subprocess.run(
                ["perl", str(AUDIT), "--root", str(root), "--paths", "prefix.cpp"],
                capture_output=True,
                text=True,
                check=False,
            )

    def test_class_size_is_not_function_size(self):
        result = self.audit("        ++root;")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn("[function-size]", result.stdout)

    def test_oversized_member_still_fails(self):
        result = self.audit("\n".join("        ++root;" for _ in range(241)))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("[fatal][function-size]", result.stdout)
        self.assertIn("member is 243 lines", result.stdout)


class FileAuditShippedFiles(unittest.TestCase):
    READS_ENVIRONMENT = '#include <cstdlib>\nconst char* home() { return std::getenv("HOME"); }\n'

    def audit(self, export_script):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            for relative in ("dev/src/lowir_model.h", "dev/src/support/testing/test_runner.cpp"):
                (root / relative).parent.mkdir(parents=True, exist_ok=True)
                (root / relative).write_text(self.READS_ENVIRONMENT)
            if export_script:
                (root / "scripts").mkdir()
                (root / "scripts/export_student_repo.sh").write_text(
                    "dev_support_files=(\n  dev/src/lowir_model.h\n)\n")
            return subprocess.run(
                ["perl", str(AUDIT), "--root", str(root), "--paths", "dev/src"],
                capture_output=True,
                text=True,
                check=False,
            )

    def test_course_skips_the_shipped_files(self):
        result = self.audit(export_script=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_export_audits_shipped_headers_but_not_the_test_runner(self):
        # The student export has no export script: its shipped headers are
        # the student's code, and only the batch test runner stays exempt.
        result = self.audit(export_script=False)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("dev/src/lowir_model.h", result.stdout)
        self.assertNotIn("test_runner.cpp", result.stdout)


if __name__ == "__main__":
    unittest.main()
