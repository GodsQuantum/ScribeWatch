#!/usr/bin/env python3
"""Exercise all desktop exports using ONLY bundled Pandoc/Typst/font assets."""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import tempfile
import zipfile

MARKDOWN = (
    "# Compte rendu — 牙科咨询\n\n"
    "- **Décision :** contrôle de la mâchoire et préparation des documents.\n"
    "- **Suivi :** réunion lundi.\n"
    "- **备注 :** 牙科咨询记录。\n"
)


def run_tools(tools: Path) -> None:
    suffix = ".exe" if tools.joinpath("pandoc.exe").is_file() else ""
    pandoc = tools / ("pandoc" + suffix)
    typst = tools / ("typst" + suffix)
    font = tools / "fonts" / "NotoSansCJK-Regular.ttc"
    manifest = tools / "EXPORT-TOOLS-SHA256.json"
    for file in (pandoc, typst, font, manifest):
        if not file.is_file():
            raise RuntimeError("Required native exporter is missing: " + str(file))
    if "pandoc 3.14" not in subprocess.check_output([str(pandoc), "--version"], text=True, timeout=25):
        raise RuntimeError("Unexpected Pandoc executable version")
    if "typst 0.15.1" not in subprocess.check_output([str(typst), "--version"], text=True, timeout=25):
        raise RuntimeError("Unexpected Typst executable version")
    with tempfile.TemporaryDirectory(prefix="scribewatch-format-smoke-") as directory:
        root = Path(directory)
        source = root / "source.md"
        source.write_text(MARKDOWN, encoding="utf-8")
        for extension, writer in (
            ("txt", "plain"), ("html", "html5"), ("docx", "docx"), ("odt", "odt"),
            ("pdf", "typst"),
        ):
            output = root / ("result." + extension)
            cmd = [
                str(pandoc), "--sandbox", "--from=gfm-raw_html", "--standalone",
                "--to=" + writer, str(source), "-o", str(output),
            ]
            if extension == "pdf":
                cmd.extend([
                    "--variable=papersize:a4",
                    "--pdf-engine=" + str(typst),
                    "--pdf-engine-opt=--font-path=" + str(tools / "fonts"),
                ])
            subprocess.run(cmd, check=True, timeout=80, capture_output=True, text=True)
            data = output.read_bytes()
            if len(data) < 120 or (extension == "pdf" and not data.startswith(b"%PDF-")):
                raise RuntimeError(f"Invalid {extension} document")
            if extension in ("docx", "odt"):
                with zipfile.ZipFile(output) as document:
                    if (extension == "docx" and "word/document.xml" not in document.namelist()) or (
                        extension == "odt" and "content.xml" not in document.namelist()
                    ):
                        raise RuntimeError(f"Missing required {extension} XML")
            elif extension in ("txt", "html"):
                text = data.decode("utf-8")
                if "contrôle de la mâchoire" not in text or "牙科咨询" not in text:
                    raise RuntimeError(f"Missing accented or CJK text in {extension}")
            print("NATIVE_PORTABLE_EXPORT_PASS", extension, len(data), flush=True)
    print("ALL_FORMATS_WITHOUT_HOST_PANDOC_WEASYPRINT_PASS", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tools-dir", type=Path, required=True)
    run_tools(parser.parse_args().tools_dir.resolve())


if __name__ == "__main__":
    main()
