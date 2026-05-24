package com.ncr.desk.api.dto;

import java.time.Instant;

public record ChatMessage(String role, String content, Instant createdAt) {}
