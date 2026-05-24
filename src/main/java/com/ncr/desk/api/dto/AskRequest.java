package com.ncr.desk.api.dto;

import com.fasterxml.jackson.annotation.JsonInclude;
import java.util.List;

@JsonInclude(JsonInclude.Include.NON_NULL)
public record AskRequest(
        String message,
        String sessionId,
        List<ChatTurn> history,
        String documentId) {}
