#!/usr/bin/env python3

import errno
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]


def host_cxx():
    return (
        os.environ.get("CPPGM_HOST_CXX")
        or os.environ.get("CXX")
        or shutil.which("g++")
        or shutil.which("clang++")
        or "g++"
    )


def run(*args, **kwargs):
    kwargs.setdefault("check", True)
    kwargs.setdefault("text", True)
    return subprocess.run(args, **kwargs)


def wait_for_file(path: Path, timeout_sec: float) -> str:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        if path.exists():
            return path.read_text().strip()
        time.sleep(0.01)
    raise AssertionError(f"timed out waiting for {path}")


def wait_for_pid_exit(pid: int, timeout_sec: float) -> None:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return
        except OSError as exc:
            if exc.errno == errno.ESRCH:
                return
            raise
        time.sleep(0.01)
    raise AssertionError(f"pid {pid} still alive after timeout")


class BatchTimeoutHarnessTests(unittest.TestCase):
    def test_text_input_contract_is_independent_of_assignment_number(self):
        profiles = [
            ("pa5", None, "first\n"),
            ("unnumbered", "sources", "first\nsecond\n"),
            ("pa50", "stdin", "first\n"),
            ("pa51", "stdin-combined", "first\ndiagnostic\n"),
        ]
        with tempfile.TemporaryDirectory(prefix="text-input-contract.") as temp_dir:
            root = Path(temp_dir)
            app = root / "tool.py"
            app.write_text(
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "if len(sys.argv) == 1:\n"
                "    print(sys.stdin.read(), end='', flush=True)\n"
                "else:\n"
                "    assert sys.argv[1] == '-o'\n"
                "    pathlib.Path(sys.argv[2]).write_text(''.join(\n"
                "        pathlib.Path(p).read_text() for p in sys.argv[3:]))\n"
                "print('diagnostic', file=sys.stderr, flush=True)\n"
            )
            app.chmod(0o755)
            for dirname, profile, expected in profiles:
                with self.subTest(profile=profile):
                    pa = root / dirname
                    tests = pa / "tests"
                    tests.mkdir(parents=True)
                    (tests / "input.t").write_text("first\n")
                    (tests / "input.t2").write_text("second\n")
                    env = os.environ.copy()
                    env.pop("CPPGM_TEXT_INPUT_PROFILE", None)
                    env["CPPGM_BATCH_TESTS"] = "0"
                    if profile:
                        env["CPPGM_TEXT_INPUT_PROFILE"] = profile
                    result = run(
                        "perl", str(REPO_ROOT / "scripts/run_all_tests_common.pl"),
                        "text_t", str(app), "my", "tests", cwd=pa, env=env,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                    )
                    self.assertEqual(result.stderr, "")
                    self.assertEqual((tests / "input.my").read_text(), expected)
                    self.assertEqual(
                        (tests / "input.my.exit_status").read_text(), "EXIT_SUCCESS\n"
                    )

    def test_run_with_timeout_reports_oom_status(self):
        result = run(
            "perl",
            str(REPO_ROOT / "scripts" / "run_with_timeout.pl"),
            "--timeout-sec",
            "10",
            "--max-rss-kb",
            "1",
            "--label",
            "oom smoke",
            "--",
            "perl",
            "-e",
            'my $x = "x" x (1024 * 1024); sleep 2',
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        self.assertEqual(result.returncode, 125)
        self.assertIn("ERROR: oom smoke OOM", result.stderr)

    def test_test_runner_timeout_kills_process_group(self):
        with tempfile.TemporaryDirectory(prefix="test-runner-batch-timeout.") as temp_dir:
            temp = Path(temp_dir)
            probe = temp / "test_runner_timeout_probe"
            source = temp / "test_runner_timeout_probe.cpp"
            source_obj = temp / "test_runner_timeout_probe.o"
            runner_obj = temp / "test_runner.o"
            pidfile = temp / "descendant.pid"
            stdout_path = temp / "request.stdout"
            stderr_path = temp / "request.stderr"

            source.write_text(
                textwrap.dedent(
                    """
                    #include <csignal>
                    #include <fstream>
                    #include <string>
                    #include <unistd.h>

                    int main(int argc, char ** argv)
                    {
                      if(argc == 3 && std::string(argv[1]) == "--hang-descendant") {
                        signal(SIGTERM, SIG_IGN);
                        const pid_t child = fork();
                        if(child < 0) {
                          return 1;
                        }
                        if(child == 0) {
                          signal(SIGTERM, SIG_IGN);
                          for(;;) {
                            pause();
                          }
                        }
                        std::ofstream pidfile(argv[2]);
                        if(!pidfile) {
                          return 1;
                        }
                        pidfile << child << std::endl;
                        pidfile.close();
                        for(;;) {
                          pause();
                        }
                      }
                      return 0;
                    }
                    """
                )
            )

            run(
                host_cxx(),
                "-std=gnu++11",
                "-Dmain=test_runner_real_main",
                "-I",
                str(REPO_ROOT / "dev" / "src"),
                "-c",
                str(source),
                "-o",
                str(source_obj),
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            run(
                host_cxx(),
                "-std=gnu++11",
                "-DTEST_RUNNER_ENABLE",
                "-I",
                str(REPO_ROOT / "dev" / "src"),
                "-c",
                str(REPO_ROOT / "dev" / "src" / "support" / "testing" / "test_runner.cpp"),
                "-o",
                str(runner_obj),
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            run(
                host_cxx(),
                str(source_obj),
                str(runner_obj),
                "-o",
                str(probe),
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            request = "\t".join(
                [
                    str(stdout_path),
                    str(stderr_path),
                    "-",
                    "CPPGM_BATCH_TIMEOUT_SEC=1",
                    "--hang-descendant",
                    str(pidfile),
                ]
            ) + "\n"

            result = run(
                "env",
                "WRAPPED_BATCH_STDIN=1",
                str(probe),
                "--batch-stdin",
                cwd=REPO_ROOT,
                input=request,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "124\n")
            self.assertEqual(result.stderr, "")
            self.assertEqual(stderr_path.read_text(), "")

            pid = int(wait_for_file(pidfile, 3.0))
            wait_for_pid_exit(pid, 3.0)

    def test_cppgm_batch_worker_timeout_kills_process_group(self):
        with tempfile.TemporaryDirectory(prefix="perl-batch-timeout.") as temp_dir:
            temp = Path(temp_dir)
            helper = temp / "hang_descendant.py"
            pidfile = temp / "descendant.pid"
            stdout_path = temp / "capture.stdout"
            stderr_path = temp / "capture.stderr"
            driver = temp / "run_capture.pl"

            helper.write_text(
                textwrap.dedent(
                    """
                    #!/usr/bin/env python3
                    import signal
                    import sys
                    import time
                    import os

                    signal.signal(signal.SIGTERM, signal.SIG_IGN)
                    child = os.fork()
                    if child == 0:
                        signal.signal(signal.SIGTERM, signal.SIG_IGN)
                        while True:
                            signal.pause()
                    with open(sys.argv[1], "w") as fh:
                        fh.write(str(child))
                    while True:
                        signal.pause()
                    """
                )
            )
            helper.chmod(0o755)

            driver.write_text(
                textwrap.dedent(
                    f"""
                    #!/usr/bin/env perl
                    use strict;
                    use warnings;
                    use lib {str(REPO_ROOT / "scripts")!r};
                    use CppgmBatchWorker qw(run_command_capture);

                    my $status = run_command_capture(
                      cmd => [{sys.executable!r}, {str(helper)!r}, {str(pidfile)!r}],
                      stdout => {str(stdout_path)!r},
                      stderr => {str(stderr_path)!r},
                      timeout => 1,
                    );
                    print "$status\\n";
                    """
                )
            )
            driver.chmod(0o755)

            result = run(
                "perl",
                str(driver),
                cwd=REPO_ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "124\n")
            self.assertEqual(result.stderr, "")

            pid = int(wait_for_file(pidfile, 3.0))
            wait_for_pid_exit(pid, 3.0)

    def test_run_all_text_test_timeout_marks_single_test_timeout(self):
        with tempfile.TemporaryDirectory(prefix="run-all-text-timeout.") as temp_dir:
            temp = Path(temp_dir) / "pa3"
            tests = temp / "tests"
            app = temp / "hang_text_test.py"
            test = tests / "hang.t"

            tests.mkdir(parents=True)
            test.write_text("input\n")
            app.write_text(
                "#!/usr/bin/env python3\n"
                "import time\n"
                "\n"
                "while True:\n"
                "    time.sleep(60)\n"
            )
            app.chmod(0o755)

            env = os.environ.copy()
            env["CPPGM_TEXT_TEST_TIMEOUT_SEC"] = "1"
            result = run(
                "perl",
                str(REPO_ROOT / "scripts" / "run_all_tests_common.pl"),
                "text_t",
                str(app),
                "my",
                "tests",
                cwd=temp,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stderr, "")
            self.assertEqual((tests / "hang.my.exit_status").read_text(), "EXIT_TIMEOUT\n")

    def test_run_all_text_test_keeps_later_assignment_output_args(self):
        with tempfile.TemporaryDirectory(prefix="run-all-text-output-args.") as temp_dir:
            temp = Path(temp_dir) / "pa5"
            tests = temp / "tests"
            app = temp / "write_output_arg.py"
            test = tests / "basic.t"

            tests.mkdir(parents=True)
            test.write_text("input\n")
            app.write_text(
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "\n"
                "if len(sys.argv) != 4 or sys.argv[1] != '-o':\n"
                "    sys.exit(2)\n"
                "with open(sys.argv[2], 'w') as fh:\n"
                "    fh.write('generated output\\n')\n"
                "print('stdout log', flush=True)\n"
                "print('stderr log', file=sys.stderr, flush=True)\n"
            )
            app.chmod(0o755)

            result = run(
                "perl",
                str(REPO_ROOT / "scripts" / "run_all_tests_common.pl"),
                "text_t",
                str(app),
                "my",
                "tests",
                cwd=temp,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stderr, "")
            self.assertEqual((tests / "basic.my.exit_status").read_text(), "EXIT_SUCCESS\n")
            self.assertEqual((tests / "basic.my").read_text(), "generated output\n")
            self.assertEqual((tests / "basic.my.stdout").read_text(), "stdout log\nstderr log\n")

    def test_failed_reference_stdout_is_kept_only_when_requested(self):
        with tempfile.TemporaryDirectory(prefix="failed-reference-stdout.") as temp_dir:
            pa = Path(temp_dir) / "pa10"
            tests = pa / "tests"
            app = pa / "reject.py"
            test = tests / "bad.t"

            tests.mkdir(parents=True)
            test.write_text("bad input\n")
            app.write_text(
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "print('host-specific diagnostic')\n"
                "sys.exit(1)\n"
            )
            app.chmod(0o755)

            command = [
                "perl",
                str(REPO_ROOT / "scripts" / "run_all_tests_common.pl"),
                "text_t",
                str(app),
                "ref",
                "tests",
            ]
            result = run(
                *command,
                cwd=pa,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.assertEqual(result.returncode, 0)
            self.assertEqual((tests / "bad.ref.exit_status").read_text(), "EXIT_FAILURE\n")
            self.assertFalse((tests / "bad.ref.stdout").exists())

            env = os.environ.copy()
            env["CPPGM_KEEP_FAILED_REFERENCE_STDOUT"] = "1"
            result = run(
                *command,
                cwd=pa,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            self.assertEqual(result.returncode, 0)
            self.assertEqual(
                (tests / "bad.ref.stdout").read_text(),
                "host-specific diagnostic\n",
            )

    def test_driver_assignment_wrapper_uses_worker_script(self):
        with tempfile.TemporaryDirectory(prefix="pa24-worker-wrapper.") as temp_dir:
            temp = Path(temp_dir)
            pa = temp / "pa24"
            tests = pa / "tests"
            app = temp / "fake_lowir_native.py"
            test = tests / "basic.t"

            tests.mkdir(parents=True)
            test.write_text("source\n")
            (tests / "basic.stdin").write_text("stdin\n")
            app.write_text(
                "#!/usr/bin/env python3\n"
                "import os\n"
                "import stat\n"
                "import sys\n"
                "\n"
                "if '--batch-stdin' not in sys.argv:\n"
                "    sys.exit(2)\n"
                "for line in sys.stdin:\n"
                "    fields = line.rstrip('\\n').split('\\t')\n"
                "    stdout_path, stderr_path, stdin_path, env_text, *args = fields\n"
                "    open(stdout_path, 'a').close()\n"
                "    open(stderr_path, 'a').close()\n"
                "    if '--dump-machine-ir' in args:\n"
                "        mir = args[args.index('--dump-machine-ir') + 1]\n"
                "        with open(mir, 'w') as fh:\n"
                "            fh.write('mir\\n')\n"
                "    program = args[args.index('-o') + 1]\n"
                "    with open(program, 'w') as fh:\n"
                "        fh.write('#!/bin/sh\\ncat\\n')\n"
                "    os.chmod(program, stat.S_IRWXU)\n"
                "    print('EXIT_SUCCESS', flush=True)\n"
            )
            app.chmod(0o755)

            result = run(
                "perl",
                str(REPO_ROOT / "pa24" / "scripts" / "run_all_tests.pl"),
                str(app),
                "my",
                "tests",
                cwd=pa,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stderr, "")
            self.assertEqual((tests / "basic.my.impl.exit_status").read_text(), "0\n")
            self.assertEqual((tests / "basic.my.program.exit_status").read_text(), "0\n")
            self.assertEqual((tests / "basic.my.program.stdout").read_text(), "stdin\n")


if __name__ == "__main__":
    unittest.main()
