package com.ncr.desk.config;

import org.springframework.boot.context.properties.ConfigurationProperties;

@ConfigurationProperties(prefix = "ai.desk")
public class DeskProperties {
    /** Rust AI desk API (knowledge base + inference orchestration). */
    private String backendUrl = "http://127.0.0.1:8090";

    public String getBackendUrl() {
        return backendUrl;
    }

    public void setBackendUrl(String backendUrl) {
        this.backendUrl = backendUrl;
    }
}
