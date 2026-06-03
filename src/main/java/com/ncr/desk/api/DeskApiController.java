package com.ncr.desk.api;

import com.ncr.desk.api.dto.ApiError;
import com.ncr.desk.api.dto.AskRequest;
import com.ncr.desk.api.dto.AskResponse;
import com.ncr.desk.api.dto.BackendHealth;
import com.ncr.desk.api.dto.DocumentDownload;
import com.ncr.desk.api.dto.DocumentUploadResponse;
import com.ncr.desk.api.dto.FrontendHealth;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.IOException;
import java.util.List;
import org.springframework.core.io.Resource;
import org.springframework.http.HttpHeaders;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.client.RestClientException;
import org.springframework.web.multipart.MultipartFile;

@RestController
@RequestMapping("/api")
public class DeskApiController {
    private static final long MAX_UPLOAD_BYTES = 10L * 1024 * 1024;

    private final AiDeskClient aiDeskClient;
    private final ObjectMapper objectMapper;

    public DeskApiController(AiDeskClient aiDeskClient, ObjectMapper objectMapper) {
        this.aiDeskClient = aiDeskClient;
        this.objectMapper = objectMapper;
    }

    @GetMapping("/health")
    public FrontendHealth health() {
        try {
            BackendHealth backend = aiDeskClient.health();
            boolean reachable = backend != null && "ok".equalsIgnoreCase(backend.status());
            boolean aiAvailable = false;
            if (backend != null && backend.llm() != null) {
                var llm = backend.llm();
                aiAvailable =
                        llm.liveAvailable()
                                || Boolean.TRUE.equals(llm.qwenAvailable())
                                || Boolean.TRUE.equals(llm.ollamaAvailable());
            }
            return new FrontendHealth(
                    reachable ? "ok" : "degraded",
                    "spring-boot",
                    reachable,
                    aiAvailable,
                    backend);
        } catch (RestClientException ex) {
            return new FrontendHealth("degraded", "spring-boot", false, false, null);
        }
    }

    @PostMapping("/ask")
    public ResponseEntity<AskResponse> ask(@RequestBody AskRequest request) {
        if (request.message() == null || request.message().isBlank()) {
            return ResponseEntity.badRequest().build();
        }
        try {
            AskRequest payload =
                    new AskRequest(
                            request.message().trim(),
                            request.sessionId(),
                            request.history() != null ? request.history() : List.of(),
                            request.documentId());
            AskResponse response = aiDeskClient.ask(payload);
            if (response == null) {
                return ResponseEntity.status(HttpStatus.BAD_GATEWAY).build();
            }
            return ResponseEntity.ok(response);
        } catch (RestClientException ex) {
            return ResponseEntity.status(HttpStatus.BAD_GATEWAY).build();
        }
    }

    @PostMapping(value = "/documents/upload", consumes = MediaType.MULTIPART_FORM_DATA_VALUE)
    public ResponseEntity<?> uploadDocument(
            @RequestParam("sessionId") String sessionId,
            @RequestParam("file") MultipartFile file) {
        if (sessionId == null || sessionId.isBlank()) {
            return ResponseEntity.badRequest().body(new ApiError("sessionId required"));
        }
        if (file == null || file.isEmpty()) {
            return ResponseEntity.badRequest().body(new ApiError("Choose a file to upload."));
        }
        if (file.getSize() > MAX_UPLOAD_BYTES) {
            return ResponseEntity.status(HttpStatus.PAYLOAD_TOO_LARGE)
                    .body(new ApiError("File too large (max 10 MB)."));
        }
        try {
            DocumentUploadResponse response = aiDeskClient.uploadDocument(sessionId.trim(), file);
            if (response == null) {
                return ResponseEntity.status(HttpStatus.BAD_GATEWAY)
                        .body(new ApiError("Document service did not respond."));
            }
            return ResponseEntity.ok(response);
        } catch (BackendApiException ex) {
            return ResponseEntity.status(resolveUploadStatus(ex.status()))
                    .body(new ApiError(extractUploadError(ex.responseBody())));
        } catch (IOException ex) {
            return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                    .body(new ApiError("Could not read the uploaded file from disk."));
        } catch (RestClientException ex) {
            return ResponseEntity.status(resolveUploadStatus(ex))
                    .body(new ApiError(extractUploadError(ex)));
        } catch (Exception ex) {
            return ResponseEntity.status(HttpStatus.INTERNAL_SERVER_ERROR)
                    .body(
                            new ApiError(
                                    "Document upload failed. Restart Spring (8080), the API"
                                            + " (8090), and the document service (8092), then try again."));
        }
    }

    private HttpStatus resolveUploadStatus(org.springframework.http.HttpStatusCode status) {
        int code = status.value();
        if (code == 413) {
            return HttpStatus.PAYLOAD_TOO_LARGE;
        }
        if (code >= 400 && code < 500) {
            return HttpStatus.BAD_REQUEST;
        }
        return HttpStatus.BAD_GATEWAY;
    }

    private HttpStatus resolveUploadStatus(RestClientException ex) {
        if (ex instanceof org.springframework.web.client.RestClientResponseException responseEx) {
            return resolveUploadStatus(responseEx.getStatusCode());
        }
        return HttpStatus.BAD_GATEWAY;
    }

    private String extractUploadError(String body) {
        if (body != null && !body.isBlank()) {
            try {
                JsonNode node = objectMapper.readTree(body);
                if (node.has("error")) {
                    return node.get("error").asText();
                }
                if (node.has("detail")) {
                    return node.get("detail").asText();
                }
            } catch (Exception ignored) {
                // fall through
            }
        }
        return "Upload failed. Ensure the document service is running on port 8092 and the API on 8090.";
    }

    private String extractUploadError(RestClientException ex) {
        if (ex instanceof org.springframework.web.client.RestClientResponseException responseEx) {
            String parsed = extractUploadError(responseEx.getResponseBodyAsString());
            if (!parsed.startsWith("Upload failed. Ensure")) {
                return parsed;
            }
        }
        String msg = ex.getMessage() != null ? ex.getMessage() : "";
        if (msg.contains("Connection refused") || msg.contains("connect")) {
            return "Cannot reach the document API on port 8090. Start the Rust API (run-api).";
        }
        if (msg.contains("timed out") || msg.contains("Timeout")) {
            return "Upload timed out. Try a smaller file or ensure the document service is running on port 8092.";
        }
        return "Upload failed. Ensure the document service is running on port 8092 and the API on 8090.";
    }

    @GetMapping("/documents/{id}/download")
    public ResponseEntity<Resource> downloadDocument(
            @PathVariable String id,
            @RequestParam("sessionId") String sessionId) {
        if (sessionId == null || sessionId.isBlank()) {
            return ResponseEntity.badRequest().build();
        }
        try {
            DocumentDownload download = aiDeskClient.downloadDocument(id, sessionId.trim());
            if (download == null || download.resource() == null) {
                return ResponseEntity.notFound().build();
            }
            return ResponseEntity.ok()
                    .contentType(MediaType.parseMediaType(download.contentType()))
                    .header(
                            HttpHeaders.CONTENT_DISPOSITION,
                            "attachment; filename=\"" + download.filename() + "\"")
                    .body(download.resource());
        } catch (RestClientException ex) {
            return ResponseEntity.status(HttpStatus.BAD_GATEWAY).build();
        }
    }
}
