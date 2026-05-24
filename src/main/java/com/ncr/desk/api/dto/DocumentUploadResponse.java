package com.ncr.desk.api.dto;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;

@JsonIgnoreProperties(ignoreUnknown = true)
public record DocumentUploadResponse(
        String documentId,
        String filename,
        String format,
        int pageCount,
        int charCount) {}
