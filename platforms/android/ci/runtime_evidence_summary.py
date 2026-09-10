#!/usr/bin/env python3
"""Summarize Android screen-reader runtime evidence as JSON and JUnit XML."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

SECRET = "android-runtime-secret"
EXPECTED_SDK = "37"


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def contains(path: Path, needle: str) -> bool:
    return needle in read_text(path)


def parse_version(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in read_text(path).splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def instrumentation_totals(root: Path) -> dict[str, int | bool]:
    patterns = (
        "app/build/outputs/androidTest-results/**/*.xml",
        "app/build/reports/androidTests/**/*.xml",
        "app/build/test-results/**/*.xml",
    )
    files: list[Path] = []
    for pattern in patterns:
        files.extend(root.glob(pattern))

    unique_files = sorted(set(files))
    tests = failures = errors = skipped = 0
    parsed_files = 0
    for path in unique_files:
        try:
            xml_root = ET.parse(path).getroot()
        except (ET.ParseError, OSError):
            continue
        if xml_root.tag not in {"testsuite", "testsuites"}:
            continue
        parsed_files += 1
        suites = [xml_root] if xml_root.tag == "testsuite" else list(xml_root.findall("testsuite"))
        for suite in suites:
            tests += int(suite.attrib.get("tests", "0") or 0)
            failures += int(suite.attrib.get("failures", "0") or 0)
            errors += int(suite.attrib.get("errors", "0") or 0)
            skipped += int(suite.attrib.get("skipped", suite.attrib.get("disabled", "0")) or 0)

    return {
        "xmlFiles": len(unique_files),
        "parsedXmlFiles": parsed_files,
        "tests": tests,
        "failures": failures,
        "errors": errors,
        "skipped": skipped,
        "passing": parsed_files > 0 and failures == 0 and errors == 0,
    }


def evaluate(root: Path, job_status: str) -> dict[str, object]:
    version_file = root / "android-version.txt"
    pre_log = root / "accessibility-logcat-pre.txt"
    post_log = root / "accessibility-logcat.txt"
    pre_enabled = root / "accessibility-enabled-pre.txt"
    post_enabled = root / "accessibility-enabled-post.txt"
    pre_touch = root / "accessibility-touch-enabled-pre.txt"
    post_touch = root / "accessibility-touch-enabled-post.txt"
    pre_dumpsys = root / "accessibility-dumpsys-pre.txt"
    post_dumpsys = root / "accessibility-dumpsys.txt"

    versions = parse_version(version_file)
    pre_text = read_text(pre_log)
    post_text = read_text(post_log)

    checks = [
        {
            "id": "android_version_evidence",
            "required": True,
            "passed": version_file.is_file(),
            "detail": "android-version.txt exists",
        },
        {
            "id": "android_sdk_37",
            "required": True,
            "passed": versions.get("ANDROID_SDK") == EXPECTED_SDK,
            "detail": f"ANDROID_SDK={versions.get('ANDROID_SDK', '')}",
        },
        {
            "id": "pre_service_enabled",
            "required": True,
            "passed": contains(pre_enabled, "org.nvdarust.screenreader"),
            "detail": "service present in enabled_accessibility_services before instrumentation",
        },
        {
            "id": "pre_touch_exploration",
            "required": True,
            "passed": read_text(pre_touch).strip() == "1",
            "detail": "touch exploration enabled before instrumentation",
        },
        {
            "id": "pre_dumpsys_service",
            "required": True,
            "passed": contains(pre_dumpsys, "org.nvdarust.screenreader"),
            "detail": "service present in dumpsys accessibility before instrumentation",
        },
        {
            "id": "pre_service_connected",
            "required": True,
            "passed": "SERVICE_CONNECTED" in pre_text,
            "detail": "AccessibilityService connected before instrumentation",
        },
        {
            "id": "pre_real_event",
            "required": True,
            "passed": "EVENT type=" in pre_text,
            "detail": "real AccessibilityEvent observed before instrumentation",
        },
        {
            "id": "pre_password_event",
            "required": True,
            "passed": "password=true" in pre_text,
            "detail": "password AccessibilityEvent observed before instrumentation",
        },
        {
            "id": "pre_password_masked",
            "required": True,
            "passed": "<password>" in pre_text and SECRET not in pre_text,
            "detail": "password data masked and fixture secret absent before instrumentation",
        },
        {
            "id": "post_service_enabled",
            "required": True,
            "passed": contains(post_enabled, "org.nvdarust.screenreader"),
            "detail": "service present in enabled_accessibility_services after instrumentation",
        },
        {
            "id": "post_touch_exploration",
            "required": True,
            "passed": read_text(post_touch).strip() == "1",
            "detail": "touch exploration enabled after instrumentation",
        },
        {
            "id": "post_dumpsys_service",
            "required": True,
            "passed": contains(post_dumpsys, "org.nvdarust.screenreader"),
            "detail": "service present in dumpsys accessibility after instrumentation",
        },
        {
            "id": "post_service_connected",
            "required": True,
            "passed": "SERVICE_CONNECTED" in post_text,
            "detail": "AccessibilityService connected after instrumentation",
        },
        {
            "id": "post_real_event",
            "required": True,
            "passed": "EVENT type=" in post_text,
            "detail": "real AccessibilityEvent observed after instrumentation",
        },
        {
            "id": "post_password_event",
            "required": True,
            "passed": "password=true" in post_text,
            "detail": "password AccessibilityEvent observed after instrumentation",
        },
        {
            "id": "post_password_masked",
            "required": True,
            "passed": "<password>" in post_text and SECRET not in post_text,
            "detail": "password data masked and fixture secret absent after instrumentation",
        },
    ]

    required = [check for check in checks if bool(check["required"])]
    overall_pass = all(bool(check["passed"]) for check in required)

    return {
        "schemaVersion": 1,
        "platform": "android",
        "expectedSdk": EXPECTED_SDK,
        "jobStatusAtSummary": job_status,
        "version": versions,
        "checks": checks,
        "instrumentation": instrumentation_totals(root),
        "requiredChecks": len(required),
        "requiredChecksPassed": sum(bool(check["passed"]) for check in required),
        "overallPass": overall_pass,
    }


def write_junit(summary: dict[str, object], output: Path) -> None:
    checks = list(summary["checks"])
    required = [check for check in checks if bool(check["required"])]
    failures = sum(not bool(check["passed"]) for check in required)
    suite = ET.Element(
        "testsuite",
        {
            "name": "android-runtime-evidence",
            "tests": str(len(required)),
            "failures": str(failures),
            "errors": "0",
            "skipped": "0",
        },
    )

    for check in required:
        case = ET.SubElement(
            suite,
            "testcase",
            {"classname": "android.runtime", "name": str(check["id"])},
        )
        if not bool(check["passed"]):
            failure = ET.SubElement(case, "failure", {"message": str(check["detail"])})
            failure.text = str(check["detail"])

    output.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(suite).write(output, encoding="utf-8", xml_declaration=True)


def populate_passing_fixture(root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "android-version.txt").write_text(
        "ANDROID_RELEASE=17\nANDROID_SDK=37\nANDROID_CODENAME=REL\n",
        encoding="utf-8",
    )
    for name in ("accessibility-enabled-pre.txt", "accessibility-enabled-post.txt"):
        (root / name).write_text(
            "org.nvdarust.screenreader/org.nvdarust.screenreader.ScreenReaderAccessibilityService\n",
            encoding="utf-8",
        )
    for name in ("accessibility-touch-enabled-pre.txt", "accessibility-touch-enabled-post.txt"):
        (root / name).write_text("1\n", encoding="utf-8")
    for name in ("accessibility-dumpsys-pre.txt", "accessibility-dumpsys.txt"):
        (root / name).write_text("org.nvdarust.screenreader\n", encoding="utf-8")
    safe_log = "SERVICE_CONNECTED\nEVENT type=32768 password=true text=<password>\n"
    (root / "accessibility-logcat-pre.txt").write_text(safe_log, encoding="utf-8")
    (root / "accessibility-logcat.txt").write_text(safe_log, encoding="utf-8")


def self_test() -> int:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        populate_passing_fixture(root)
        passing = evaluate(root, "success")
        if not passing["overallPass"]:
            raise AssertionError("passing fixture did not pass")

        with (root / "accessibility-logcat.txt").open("a", encoding="utf-8") as stream:
            stream.write(f"leak={SECRET}\n")
        leaking = evaluate(root, "success")
        by_id = {str(check["id"]): check for check in leaking["checks"]}
        if bool(by_id["post_password_masked"]["passed"]):
            raise AssertionError("secret leak was not detected")
        if leaking["overallPass"]:
            raise AssertionError("leaking fixture incorrectly passed")

    print("ANDROID_RUNTIME_EVIDENCE_SUMMARY_SELF_TEST = PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("platforms/android"))
    parser.add_argument("--json", type=Path, default=Path("platforms/android/runtime-summary.json"))
    parser.add_argument("--junit", type=Path, default=Path("platforms/android/runtime-summary.junit.xml"))
    parser.add_argument("--job-status", default="unknown")
    parser.add_argument("--enforce-on-success", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    summary = evaluate(args.root, args.job_status)
    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_junit(summary, args.junit)

    print(
        "ANDROID_RUNTIME_EVIDENCE_SUMMARY = "
        f"{'PASS' if summary['overallPass'] else 'FAIL'} "
        f"({summary['requiredChecksPassed']}/{summary['requiredChecks']} required checks)"
    )

    if args.enforce_on_success and args.job_status == "success" and not summary["overallPass"]:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
