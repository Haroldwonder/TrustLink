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
