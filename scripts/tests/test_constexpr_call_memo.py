"""Constant-call reuse must preserve contexts, failures and resource limits."""

import pathlib
import re
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
COMPILER = ROOT / "dev" / "cppgm++"


@unittest.skipUnless(COMPILER.is_file(), "build dev/cppgm++ first")
class ConstantCallMemoTests(unittest.TestCase):
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
        return result

    def test_overlapping_recursion_reuses_successful_values(self):
        result = self.compile("""
constexpr int fib(int n) { return n < 2 ? n : fib(n-1) + fib(n-2); }
constexpr int n = fib(22);
static_assert(n == 17711, "result"); int main() { return 0; }
""")
        counters = dict(re.findall(r"(constant_call_\w+)=(\d+)", result.stderr))
        self.assertGreater(int(counters["constant_call_reuses"]), 0)
        self.assertLess(int(counters["constant_call_builds"]), 100)

    def test_mutable_reference_calls_are_not_reused(self):
        self.compile("""
constexpr int bump(int& n) { return ++n; }
constexpr int run() { int n=0; int a=bump(n); int b=bump(n); return a*10+b; }
static_assert(run()==12, "distinct state"); int main() { return 0; }
""")

    def test_failed_argument_does_not_reuse_a_success(self):
        self.compile("""
constexpr int inverse(int n) { return 12/n; }
constexpr int n = inverse(3) + inverse(0); int main() { return n; }
""", succeeds=False)

    def test_reused_call_still_evaluates_its_arguments(self):
        result = self.compile("""
constexpr int pure(int n) { return n+10; }
constexpr int run() {
    int n=0;
    int a=pure((++n, 1));
    int b=pure((++n, 1));
    return a+b+n;
}
static_assert(run()==24, "argument effects on a cache hit");
int main() { return 0; }
""")
        self.assertRegex(result.stderr, r"constant_call_reuses=[1-9]")

    def test_callee_expression_is_not_assumed_pure(self):
        self.compile("""
constexpr int plain(int n) { return n; }
constexpr int read(int n) { return (++n, plain)(7); }
static_assert(read(1)+read(1)==14, "callee expression effects");
int main() { return 0; }
""")

    def test_wide_arguments_keep_their_high_bits(self):
        self.compile("""
constexpr unsigned long long high(unsigned __int128 n) {
    return (unsigned long long)(n >> 64);
}
constexpr auto n = high((unsigned __int128)1 << 64) + high((unsigned __int128)2 << 64);
static_assert(n == 3, "distinct high words"); int main() { return 0; }
""")

    def test_cached_result_keeps_the_call_depth_charge(self):
        result = self.compile("""
constexpr int descend(int n) { return n ? descend(n-1)+1 : 0; }
constexpr int outer(int n) { return n ? outer(n-1)+1 : descend(500); }
constexpr int n = (descend(500), outer(32)); int main() { return n; }
""", succeeds=False)
        self.assertIn("constexpr calls nested too deeply", result.stderr)

    def test_cached_result_keeps_the_step_budget_charge(self):
        result = self.compile("""
constexpr int fib(int n) { return n < 2 ? n : fib(n-1)+fib(n-2); }
constexpr int n = fib(31); int main() { return n; }
""", succeeds=False)
        self.assertIn("constant evaluation exceeds its step budget", result.stderr)

    def test_many_distinct_calls_saturate_without_changing_results(self):
        result = self.compile("""
constexpr int tree(int n) { return n>3000 ? n : tree(n*2)+tree(n*2+1); }
static_assert(tree(1)==13507501, "all leaves after saturation");
int main() { return 0; }
""")
        counters = dict(re.findall(r"(constant_call_\w+)=(\d+)", result.stderr))
        self.assertEqual(int(counters["constant_call_builds"]), 4096)


if __name__ == "__main__":
    unittest.main()
