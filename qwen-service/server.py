"""
Document extract/export and embeddings for NCR AI Desk (no local LLM).
LLM answers use Perplexity Sonar via the Rust API (live web data).
"""

from __future__ import annotations

import os
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI, File, HTTPException, UploadFile
from fastapi.responses import Response
from pydantic import BaseModel, Field

from document_tools import (
    export_document,
    extract_document,
    extract_text_from_pdf,
    generate_pdf_bytes,
)

BIND_HOST = os.environ.get("DOCUMENT_BIND_HOST", os.environ.get("QWEN_BIND_HOST", "127.0.0.1"))
BIND_PORT = int(os.environ.get("DOCUMENT_BIND_PORT", os.environ.get("QWEN_BIND_PORT", "8092")))

embedder = None


class EmbedRequest(BaseModel):
    texts: list[str]


class EmbedResponse(BaseModel):
    vectors: list[list[float]]
    model: str = "sentence-transformers/all-MiniLM-L6-v2"
    dimensions: int = 384


class PdfGenerateRequest(BaseModel):
    title: str = "Document"
    body: str
    filename: str | None = None


class PdfExtractResponse(BaseModel):
    text: str
    pageCount: int
    charCount: int


class DocumentExtractResponse(BaseModel):
    text: str
    pageCount: int
    charCount: int
    format: str


class DocumentExportRequest(BaseModel):
    title: str = "Document"
    body: str
    filename: str | None = None
    format: str | None = None


@asynccontextmanager
async def lifespan(_app: FastAPI):
    global embedder
    from fastembed import TextEmbedding

    print("Loading embeddings (fastembed)…")
    embedder = TextEmbedding(model_name="sentence-transformers/all-MiniLM-L6-v2")
    print("Document service ready (PDF/Word + embeddings). LLM: Perplexity via Rust API.")
    yield


MAX_UPLOAD_BYTES = 10 * 1024 * 1024

app = FastAPI(title="NCR document service", lifespan=lifespan)


@app.get("/health")
def health():
    return {
        "status": "ok",
        "ready": embedder is not None,
        "embeddingsReady": embedder is not None,
        "llm": "perplexity (configured in Rust API)",
    }


@app.post("/embed", response_model=EmbedResponse)
def embed(req: EmbedRequest):
    if embedder is None:
        raise HTTPException(status_code=503, detail="Embedding model not loaded")
    if not req.texts:
        raise HTTPException(status_code=400, detail="texts required")
    vectors = [vec.tolist() for vec in embedder.embed(req.texts)]
    return EmbedResponse(vectors=vectors)


@app.post("/pdf/extract", response_model=PdfExtractResponse)
async def pdf_extract(file: UploadFile = File(...)):
    if not file.filename or not file.filename.lower().endswith(".pdf"):
        raise HTTPException(status_code=400, detail="Upload a .pdf file")
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="Empty file")
    if len(data) > MAX_UPLOAD_BYTES:
        raise HTTPException(status_code=413, detail="PDF too large (max 10 MB)")
    try:
        result = extract_text_from_pdf(data)
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Could not read PDF: {exc}") from exc
    if not result["text"].strip():
        raise HTTPException(
            status_code=400,
            detail="No extractable text in this PDF (it may be scanned images only).",
        )
    return PdfExtractResponse(**result)


@app.post("/documents/extract", response_model=DocumentExtractResponse)
async def documents_extract(file: UploadFile = File(...)):
    if not file.filename:
        raise HTTPException(status_code=400, detail="Filename required")
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="Empty file")
    if len(data) > MAX_UPLOAD_BYTES:
        raise HTTPException(status_code=413, detail="File too large (max 10 MB)")
    try:
        result = extract_document(data, file.filename)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=400, detail=f"Could not read file: {exc}") from exc
    if not result["text"].strip():
        raise HTTPException(status_code=400, detail="No extractable text in this file.")
    return DocumentExtractResponse(**result)


@app.post("/documents/export")
def documents_export(req: DocumentExportRequest):
    if not req.body.strip():
        raise HTTPException(status_code=400, detail="body required")
    try:
        content, name, mime = export_document(
            req.title, req.body, req.filename or "document.txt", req.format
        )
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Export failed: {exc}") from exc
    return Response(
        content=content,
        media_type=mime,
        headers={"Content-Disposition": f'attachment; filename="{name}"'},
    )


@app.post("/pdf/generate")
def pdf_generate(req: PdfGenerateRequest):
    if not req.body.strip():
        raise HTTPException(status_code=400, detail="body required")
    try:
        pdf_bytes = generate_pdf_bytes(req.title, req.body)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"PDF generation failed: {exc}") from exc
    name = (req.filename or "document.pdf").strip()
    if not name.lower().endswith(".pdf"):
        name += ".pdf"
    return Response(
        content=pdf_bytes,
        media_type="application/pdf",
        headers={"Content-Disposition": f'attachment; filename="{name}"'},
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host=BIND_HOST, port=BIND_PORT, log_level="info")
