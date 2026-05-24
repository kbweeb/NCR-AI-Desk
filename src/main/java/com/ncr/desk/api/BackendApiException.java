package com.ncr.desk.api;

import org.springframework.http.HttpStatusCode;

/** Rust document API returned a non-success status. */
public class BackendApiException extends RuntimeException {
    private final HttpStatusCode status;
    private final String responseBody;

    public BackendApiException(HttpStatusCode status, String responseBody) {
        super("Backend API " + status.value());
        this.status = status;
        this.responseBody = responseBody != null ? responseBody : "";
    }

    public HttpStatusCode status() {
        return status;
    }

    public String responseBody() {
        return responseBody;
    }
}
