package com.ncr.desk.api.dto;

import java.time.Instant;

public record ChatSummary(
        String id, String title, String preview, Instant updatedAt, int messageCount) {}
