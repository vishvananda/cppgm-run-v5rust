"""Function ordering is structural, with separate call and address forms."""

import pathlib
import re
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPILER = ROOT / "dev" / "cppgm++"


@unittest.skipUnless(COMPILER.is_file(), "build dev/cppgm++ first")
class FunctionOrderReuseTests(unittest.TestCase):
    def compile(self, source, succeeds=True):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "input.cpp"
            path.write_text(source)
            result = subprocess.run(
                [str(COMPILER), "--emit-lowir", "-O0", "--stats", "-o",
                 str(path.with_suffix(".lowir")), str(path)],
                capture_output=True, text=True, timeout=30,
            )
        self.assertEqual(result.returncode, 0 if succeeds else 1, result.stderr)
        return {key: int(value) for key, value in re.findall(
            r"(function_order_\w+)=(\d+)", result.stderr)}

    def ordering_family(self, count):
        source = """
template<class T> constexpr int choose(T) { return 1; }
template<class T> constexpr int choose(T*) { return 2; }
template<class T> constexpr int choose(const T*) { return 3; }
"""
        for index in range(count):
            source += f"""
struct X{index} {{}};
static_assert(choose((X{index}*)0) == 2, "pointer");
static_assert(choose((const X{index}*)0) == 3, "const pointer");
"""
        return self.compile(source + "int main() { return 0; }")

    def test_new_argument_types_reuse_structural_comparisons(self):
        small, large = self.ordering_family(4), self.ordering_family(40)
        self.assertGreater(small["function_order_builds"], 0)
        self.assertEqual(small["function_order_builds"], large["function_order_builds"])
        self.assertGreater(large["function_order_reuses"], small["function_order_reuses"])

    def test_calls_addresses_defaults_and_redeclarations(self):
        self.compile("""
template<class T> constexpr int choose(T) { return 1; }
template<class T> constexpr int choose(T*) { return 2; }
static_assert(choose((int*)0)==2, "call before address");
int (*address)(int*) = choose;
template<class U> constexpr int choose(U*);
static_assert(choose((long*)0)==2, "redeclared head");
template<class T> constexpr int other(T, int = 0) { return 3; }
template<class T> constexpr int other(T*, int = 0) { return 4; }
static_assert(other((int*)0)==4, "one argument");
static_assert(other((int*)0, 1)==4, "two arguments");
int main() { return address((int*)0)==2 ? 0 : 1; }
""")

    def test_prior_ordering_does_not_resolve_an_ambiguous_call(self):
        self.compile("""
template<class T> constexpr int choose(T*) { return 1; }
template<class T> constexpr int choose(const T*) { return 2; }
static_assert(choose((int*)0)==1, "first call");
template<class T> constexpr int choose(volatile T*) { return 3; }
constexpr int bad = choose((const volatile int*)0);
int main() { return bad; }
""", succeeds=False)


if __name__ == "__main__":
    unittest.main()
