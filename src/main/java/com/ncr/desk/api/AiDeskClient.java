package com.ncr.desk.api;

import com.ncr.desk.api.dto.AskRequest;
import com.ncr.desk.api.dto.AskResponse;
import com.ncr.desk.api.dto.BackendHealth;
import com.ncr.desk.api.dto.DocumentDownload;
import com.ncr.desk.api.dto.DocumentUploadResponse;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.ncr.desk.config.DeskProperties;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import org.springframework.boot.web.client.ClientHttpRequestFactories;
import org.springframework.boot.web.client.ClientHttpRequestFactorySettings;
import org.springframework.core.io.ByteArrayResource;
import org.springframework.core.io.Resource;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.http.client.MultipartBodyBuilder;
import org.springframework.web.client.RestClient;
import org.springframework.web.client.RestClientException;
import org.springframework.web.multipart.MultipartFile;

public class AiDeskClient {
    private final RestClient healthClient;
    private final RestClient apiClient;
    private final ObjectMapper objectMapper;

    public AiDeskClient(DeskProperties properties, ObjectMapper objectMapper) {
        this.objectMapper = objectMapper;
        String base = properties.getBackendUrl();
        var healthFactory =
                ClientHttpRequestFactories.get(
                        ClientHttpRequestFactorySettings.DEFAULTS
                                .withConnectTimeout(Duration.ofSeconds(2))
                                .withReadTimeout(Duration.ofSeconds(8)));
        var apiFactory =
                ClientHttpRequestFactories.get(
                        ClientHttpRequestFactorySettings.DEFAULTS
                                .withConnectTimeout(Duration.ofSeconds(3))
                                .withReadTimeout(Duration.ofSeconds(120)));

        this.healthClient =
                RestClient.builder()
                        .baseUrl(base)
                        .requestFactory(healthFactory)
                        .build();
        this.apiClient =
                RestClient.builder().baseUrl(base).requestFactory(apiFactory).build();
    }

    public BackendHealth health() {
        return healthClient.get().uri("/health").retrieve().body(BackendHealth.class);
    }

    public boolean isReachable() {
        try {
            BackendHealth health = health();
            return health != null && "ok".equalsIgnoreCase(health.status());
        } catch (RestClientException ex) {
            return false;
        }
    }

    public AskResponse ask(AskRequest request) {
        return apiClient
                .post()
                .uri("/api/ask")
                .contentType(MediaType.APPLICATION_JSON)
                .body(request)
                .retrieve()
                .body(AskResponse.class);
    }

    public DocumentUploadResponse uploadDocument(String sessionId, MultipartFile file)
            throws IOException {
        byte[] bytes = file.getBytes();
        String filename =
                file.getOriginalFilename() != null && !file.getOriginalFilename().isBlank()
                        ? file.getOriginalFilename()
                        : "upload.bin";
        MediaType fileType = resolveUploadMediaType(filename, file.getContentType());

        MultipartBodyBuilder builder = new MultipartBodyBuilder();
        builder.part("sessionId", sessionId);
        builder
                .part(
                        "file",
                        new ByteArrayResource(bytes) {
                            @Override
                            public String getFilename() {
                                return filename;
                            }
                        })
                .filename(filename)
                .contentType(fileType);

        return apiClient
                .post()
                .uri("/api/documents/upload")
                .body(builder.build())
                .exchange(
                        (request, response) -> {
                            byte[] raw = response.bodyTo(byte[].class);
                            byte[] body = raw != null ? raw : new byte[0];
                            if (response.getStatusCode().is2xxSuccessful()) {
                                if (body.length == 0) {
                                    throw new IOException("Document API returned an empty body.");
                                }
                                try {
                                    return objectMapper.readValue(body, DocumentUploadResponse.class);
                                } catch (IOException parseEx) {
                                    throw new IOException("Invalid document API response.", parseEx);
                                }
                            }
                            String text = new String(body, StandardCharsets.UTF_8);
                            throw new BackendApiException(response.getStatusCode(), text);
                        });
    }

    private static MediaType resolveUploadMediaType(String filename, String probe) {
        if (probe != null && !probe.isBlank() && !MediaType.APPLICATION_OCTET_STREAM_VALUE.equals(probe)) {
            return MediaType.parseMediaType(probe);
        }
        String lower = filename.toLowerCase();
        if (lower.endsWith(".pdf")) {
            return MediaType.APPLICATION_PDF;
        }
        if (lower.endsWith(".docx")) {
            return MediaType.parseMediaType(
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
        }
        if (lower.endsWith(".txt") || lower.endsWith(".md") || lower.endsWith(".csv")) {
            return MediaType.TEXT_PLAIN;
        }
        return MediaType.APPLICATION_OCTET_STREAM;
    }

    private static final Pattern FILENAME_IN_DISPOSITION =
            Pattern.compile("filename\\*?=(?:UTF-8''|\"?)([^\";]+)", Pattern.CASE_INSENSITIVE);

    public DocumentDownload downloadDocument(String documentId, String sessionId) {
        ResponseEntity<byte[]> response =
                apiClient
                        .get()
                        .uri(
                                uriBuilder ->
                                        uriBuilder
                                                .path("/api/documents/{id}/download")
                                                .queryParam("sessionId", sessionId)
                                                .build(documentId))
                        .retrieve()
                        .toEntity(byte[].class);
        byte[] bytes = response.getBody();
        if (bytes == null) {
            return null;
        }
        HttpHeaders headers = response.getHeaders();
        String contentType =
                headers.getContentType() != null
                        ? headers.getContentType().toString()
                        : MediaType.APPLICATION_OCTET_STREAM_VALUE;
        String filename = "document";
        String disposition = headers.getFirst(HttpHeaders.CONTENT_DISPOSITION);
        if (disposition != null) {
            Matcher matcher = FILENAME_IN_DISPOSITION.matcher(disposition);
            if (matcher.find()) {
                filename = matcher.group(1).trim();
            }
        }
        return new DocumentDownload(new ByteArrayResource(bytes), contentType, filename);
    }
}
