package com.ncr.desk.persistence;

import jakarta.persistence.Column;
import jakarta.persistence.Entity;
import jakarta.persistence.GeneratedValue;
import jakarta.persistence.GenerationType;
import jakarta.persistence.Id;
import jakarta.persistence.Table;
import java.time.Instant;

@Entity
@Table(name = "messages")
public class MessageEntity {
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(nullable = false, length = 36)
    private String conversationId;

    @Column(nullable = false, length = 16)
    private String role;

    @Column(nullable = false, length = 16000)
    private String content;

    @Column(nullable = false)
    private int sortOrder;

    @Column(nullable = false)
    private Instant createdAt;

    protected MessageEntity() {}

    public MessageEntity(
            String conversationId, String role, String content, int sortOrder, Instant createdAt) {
        this.conversationId = conversationId;
        this.role = role;
        this.content = content;
        this.sortOrder = sortOrder;
        this.createdAt = createdAt;
    }

    public Long getId() {
        return id;
    }

    public String getConversationId() {
        return conversationId;
    }

    public String getRole() {
        return role;
    }

    public String getContent() {
        return content;
    }

    public int getSortOrder() {
        return sortOrder;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }
}
