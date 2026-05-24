package com.ncr.desk.api.dto;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

@JsonIgnoreProperties(ignoreUnknown = true)
public record BackendHealth(
        String status,
        String service,
        String version,
        LlmHealth llm) {

    @JsonIgnoreProperties(ignoreUnknown = true)
    public record LlmHealth(
            String mode,
            boolean liveAvailable,
            String liveModel,
            boolean documentServiceAvailable,
            String documentServiceUrl,
            // Legacy fields (older API builds)
            Boolean qwenAvailable,
            Boolean ollamaAvailable) {}
}
