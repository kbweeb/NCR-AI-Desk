package com.ncr.desk.api.dto;

import java.time.Instant;
import java.util.List;

public record ChatDetail(
        String id,
        String title,
        String sessionId,
        String documentId,
        String documentName,
        Instant createdAt,
        Instant updatedAt,
        List<ChatMessage> messages) {}
