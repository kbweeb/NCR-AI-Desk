package com.ncr.desk.api;

import com.ncr.desk.api.dto.ApiError;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.RestControllerAdvice;
import org.springframework.web.multipart.MaxUploadSizeExceededException;
import org.springframework.web.multipart.MultipartException;

@RestControllerAdvice
public class DeskExceptionHandler {
    private static final String MAX_UPLOAD_MSG = "File too large (max 10 MB).";

    @ExceptionHandler(MaxUploadSizeExceededException.class)
    public ResponseEntity<ApiError> handleMaxUpload(MaxUploadSizeExceededException ex) {
        return ResponseEntity.status(HttpStatus.PAYLOAD_TOO_LARGE).body(new ApiError(MAX_UPLOAD_MSG));
    }

    @ExceptionHandler(MultipartException.class)
    public ResponseEntity<ApiError> handleMultipart(MultipartException ex) {
        Throwable cause = ex.getCause();
        if (cause instanceof MaxUploadSizeExceededException) {
            return handleMaxUpload((MaxUploadSizeExceededException) cause);
        }
        String msg = ex.getMessage() != null ? ex.getMessage() : "Upload failed.";
        if (msg.toLowerCase().contains("size") || msg.toLowerCase().contains("limit")) {
            return ResponseEntity.status(HttpStatus.PAYLOAD_TOO_LARGE)
                    .body(new ApiError(MAX_UPLOAD_MSG));
        }
        return ResponseEntity.status(HttpStatus.BAD_REQUEST)
                .body(new ApiError("Could not read the upload. Try a smaller file (max 10 MB)."));
    }

}
