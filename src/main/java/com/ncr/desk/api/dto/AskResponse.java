package com.ncr.desk.api.dto;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import java.util.List;

@JsonIgnoreProperties(ignoreUnknown = true)
public record AskResponse(
        String reply,
        String intent,
        float confidence,
        String engine,
        List<SourceRef> sources,
        List<String> suggestedFollowUps,
        DocumentArtifact documentArtifact,
        String activeDocumentId,
        String documentEditPreview) {

    @JsonIgnoreProperties(ignoreUnknown = true)
    public record SourceRef(String id, String title, String category, float score) {}
}
