#!/usr/bin/env python3
"""Install a student bundle in a fresh client and exercise its reference wrappers."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def check_documents(export: Path) -> None:
    root = Path(__file__).resolve().parents[1]
    expected = set((root / 'scripts/student_export_documents.txt').read_text().splitlines())
    expected.update(path.name for path in (root / 'docs/student-export-root').glob('*.md'))
    actual = {str(path.relative_to(export)) for path in export.rglob('*')
              if path.is_file() and (path.suffix.lower() == '.md'
                                    or path.relative_to(export).parts[0] == 'doc')}
    if actual != expected:
        raise RuntimeError(f"student documents: missing {sorted(expected - actual)}; "
                           f"unexpected {sorted(actual - expected)}")
    print(f"Student document inventory: {len(actual)} files, PASS")


def check_export(export: Path, bundle: Path | None) -> None:
    check_documents(export)
    env = os.environ.copy()
    # A release check must follow the published manifest URL, with no local cache.
    env.pop("CPPGM_REFERENCE_BUNDLE_FILE", None)
    env.pop("CPPGM_REFERENCE_BUNDLE_URL", None)
    if bundle is not None:
        env["CPPGM_REFERENCE_BUNDLE_FILE"] = str(bundle.resolve(strict=True))

    with tempfile.TemporaryDirectory(prefix="cppgm-reference-client.") as temp:
        client = Path(temp)
        for relative in ("reference-binaries/manifest.tsv",
                         "scripts/ensure_reference_binaries.pl",
                         "scripts/run_reference_binary.sh"):
            target = client / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(export / relative, target)

        wrappers = sorted(export.glob("dev/*-ref")) + sorted(export.glob("pa*/*-ref"))
        if not wrappers:
            raise RuntimeError("export has no reference wrappers")
        for wrapper in wrappers:
            target = client / wrapper.relative_to(export)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(wrapper, target, follow_symlinks=False)

        def run(*args: object, input: str = "") -> None:
            subprocess.run([str(arg) for arg in args], input=input, text=True,
                           stdout=subprocess.DEVNULL, check=True, env=env,
                           cwd=client, timeout=120)

        run("perl", client / "scripts/ensure_reference_binaries.pl")
        for binary in (client / "reference-binaries").iterdir():
            if binary.name == "manifest.tsv":
                continue
            dynamic = subprocess.check_output(
                ["readelf", "-d", str(binary)], text=True, timeout=30)
            if "libjemalloc.so" in dynamic:
                raise RuntimeError(f"{binary.name} depends on optional host jemalloc")
        source = client / "smoke.cpp"
        source.write_text("int main() { return 0; }\n")
        for wrapper in wrappers:
            installed = client / wrapper.relative_to(export)
            name = wrapper.name.removesuffix("-ref")
            if name in {"pptoken", "posttoken", "ppexpr"}:
                run(installed, input="1\n")
            elif name == "preproc":
                run(installed, "-o", client / "smoke.tokens", source)
            else:
                run(installed, "--help")

        # Exercise the renamed LowIR tool and its supplied backend together.
        for fixture in sorted((export / "pa8/tests/behavior").glob("*.exercise")):
            exercise = fixture.read_text().strip()
            implementation = client / f"{exercise}.lowir"
            program = client / f"{exercise}.program"
            run(client / "pa8/lowir-ref", "--exercise", exercise,
                "-o", implementation)
            run(client / "pa24/lowir2native-ref", "-o", program,
                implementation, fixture.with_suffix(".t"))
            run(program)

        print(f"Reference bundle install and {len(wrappers)} wrappers: PASS")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("--bundle", type=Path,
                        help="use a local archive; otherwise download the published URL")
    args = parser.parse_args()
    check_export(args.export.resolve(strict=True), args.bundle)
