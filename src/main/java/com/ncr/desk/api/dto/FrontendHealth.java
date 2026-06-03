package com.ncr.desk.api.dto;

public record FrontendHealth(
        String status,
        String frontend,
        boolean backendReachable,
        boolean aiAvailable,
        boolean documentsAvailable,
        BackendHealth backend) {}
