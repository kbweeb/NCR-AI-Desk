package com.ncr.desk.api.dto;

import org.springframework.core.io.Resource;

public record DocumentDownload(Resource resource, String contentType, String filename) {}
