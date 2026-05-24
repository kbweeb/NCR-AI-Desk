package com.ncr.desk.api.dto;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

@JsonIgnoreProperties(ignoreUnknown = true)
public record DocumentArtifact(
        String documentId,
        String filename,
        String format,
        String mimeType,
        int revision,
        String downloadUrl) {}
