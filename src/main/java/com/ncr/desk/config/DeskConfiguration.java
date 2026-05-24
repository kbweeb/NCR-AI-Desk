package com.ncr.desk.config;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.ncr.desk.api.AiDeskClient;
import org.springframework.boot.context.properties.EnableConfigurationProperties;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
@EnableConfigurationProperties(DeskProperties.class)
public class DeskConfiguration {
    @Bean
    AiDeskClient aiDeskClient(DeskProperties properties, ObjectMapper objectMapper) {
        return new AiDeskClient(properties, objectMapper);
    }
}
