"""Packaging validation tests for trustlink-sdk."""

import importlib.metadata
import zipfile
from pathlib import Path


def test_py_typed_marker_exists() -> None:
    """py.typed must be present in the source tree (PEP 561)."""
    marker = Path(__file__).parent.parent / "trustlink" / "py.typed"
    assert marker.exists(), "trustlink/py.typed marker file is missing"


def test_public_api_exports() -> None:
    """All advertised public symbols must be importable from the top-level package."""
    import importlib
    import pytest

    try:
        pkg = importlib.import_module("trustlink")
    except ImportError as exc:
        pytest.skip(f"trustlink not importable in this environment: {exc}")

    expected = [
        "TrustLinkClient",
        "AsyncTrustLinkClient",
        "Attestation",
        "AttestationStatus",
        "ClaimTypeInfo",
        "GlobalStats",
        "IssuerStats",
        "MultiSigProposal",
        "TrustLinkError",
        "ContractError",
    ]
    missing = [name for name in expected if not hasattr(pkg, name)]
    assert not missing, f"Missing public exports: {missing}"


def test_wheel_contains_py_typed(tmp_path: Path) -> None:
    """Built wheel must include trustlink/py.typed (PEP 561)."""
    import subprocess, sys

    pkg_dir = Path(__file__).parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "build", "--wheel", "--outdir", str(tmp_path)],
        cwd=pkg_dir,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"build failed:\n{result.stderr}"

    wheels = list(tmp_path.glob("*.whl"))
    assert wheels, "No wheel produced"

    with zipfile.ZipFile(wheels[0]) as whl:
        names = whl.namelist()

    typed_files = [n for n in names if n.endswith("py.typed")]
    assert typed_files, f"py.typed not found in wheel. Contents:\n{names}"


def test_wheel_metadata_name(tmp_path: Path) -> None:
    """Wheel METADATA must declare the package name as trustlink-sdk."""
    import subprocess, sys

    pkg_dir = Path(__file__).parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "build", "--wheel", "--outdir", str(tmp_path)],
        cwd=pkg_dir,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"build failed:\n{result.stderr}"

    wheels = list(tmp_path.glob("*.whl"))
    assert wheels

    with zipfile.ZipFile(wheels[0]) as whl:
        metadata_path = next(n for n in whl.namelist() if n.endswith("/METADATA"))
        metadata = whl.read(metadata_path).decode()

    assert "Name: trustlink-sdk" in metadata, (
        f"Expected 'Name: trustlink-sdk' in METADATA, got:\n{metadata[:500]}"
    )


def test_wheel_excludes_misspelled_bundle_helpers(tmp_path: Path) -> None:
    """Wheel must not ship the misspelled trustlink/bundlle_helpers.py duplicate."""
    import subprocess, sys

    pkg_dir = Path(__file__).parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "build", "--wheel", "--outdir", str(tmp_path)],
        cwd=pkg_dir,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"build failed:\n{result.stderr}"

    wheels = list(tmp_path.glob("*.whl"))
    assert wheels, "No wheel produced"

    with zipfile.ZipFile(wheels[0]) as whl:
        names = whl.namelist()

    misspelled = [n for n in names if n.endswith("trustlink/bundlle_helpers.py")]
    assert not misspelled, (
        f"Misspelled duplicate module found in wheel: {misspelled}"
    )
    correct = [n for n in names if n.endswith("trustlink/bundle_helpers.py")]
    assert correct, f"bundle_helpers.py not found in wheel. Contents:\n{names}"


def test_wheel_requires_python(tmp_path: Path) -> None:
    """Wheel METADATA must declare Requires-Python >=3.10."""
    import subprocess, sys

    pkg_dir = Path(__file__).parent.parent
    result = subprocess.run(
        [sys.executable, "-m", "build", "--wheel", "--outdir", str(tmp_path)],
        cwd=pkg_dir,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, f"build failed:\n{result.stderr}"

    wheels = list(tmp_path.glob("*.whl"))
    assert wheels, "No wheel produced"

    with zipfile.ZipFile(wheels[0]) as whl:
        metadata_path = next(n for n in whl.namelist() if n.endswith("/METADATA"))
        metadata = whl.read(metadata_path).decode()

    requires_python = [
        line for line in metadata.splitlines() if line.startswith("Requires-Python:")
    ]
    assert requires_python, (
        f"Requires-Python not found in METADATA, got:\n{metadata[:500]}"
    )
    spec = requires_python[0].split(":", 1)[1].strip()
    assert "3.10" in spec, (
        f"Expected Requires-Python to require >=3.10, got: {spec!r}"
    )
    assert "3.8" not in spec and "3.9" not in spec, (
        f"Requires-Python still allows end-of-life versions: {spec!r}"
    )
