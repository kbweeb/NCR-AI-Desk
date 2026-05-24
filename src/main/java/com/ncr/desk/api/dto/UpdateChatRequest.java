package com.ncr.desk.api.dto;

public record UpdateChatRequest(
        String userId, String title, String documentId, String documentName) {}
