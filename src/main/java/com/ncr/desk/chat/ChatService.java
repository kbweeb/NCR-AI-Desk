package com.ncr.desk.chat;

import com.ncr.desk.api.dto.AppendChatMessageRequest;
import com.ncr.desk.api.dto.ChatDetail;
import com.ncr.desk.api.dto.ChatMessage;
import com.ncr.desk.api.dto.ChatSummary;
import com.ncr.desk.api.dto.UpdateChatRequest;
import com.ncr.desk.persistence.ConversationEntity;
import com.ncr.desk.persistence.ConversationRepository;
import com.ncr.desk.persistence.MessageEntity;
import com.ncr.desk.persistence.MessageRepository;
import java.time.Instant;
import java.util.List;
import java.util.UUID;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

@Service
public class ChatService {
    private static final int TITLE_MAX = 80;
    private static final int PREVIEW_MAX = 120;

    private final ConversationRepository conversations;
    private final MessageRepository messages;

    public ChatService(ConversationRepository conversations, MessageRepository messages) {
        this.conversations = conversations;
        this.messages = messages;
    }

    @Transactional(readOnly = true)
    public List<ChatSummary> listForUser(String userId) {
        return conversations.findByUserIdOrderByUpdatedAtDesc(userId).stream()
                .map(this::toSummary)
                .toList();
    }

    @Transactional(readOnly = true)
    public ChatDetail getForUser(String chatId, String userId) {
        ConversationEntity conv = requireConversation(chatId, userId);
        return toDetail(conv);
    }

    @Transactional
    public ChatDetail create(String userId, String title) {
        Instant now = Instant.now();
        String id = UUID.randomUUID().toString();
        String sessionId = UUID.randomUUID().toString();
        String resolvedTitle =
                (title != null && !title.isBlank()) ? trimTitle(title) : "New chat";
        ConversationEntity conv =
                new ConversationEntity(id, userId, resolvedTitle, sessionId, now, now);
        conversations.save(conv);
        return toDetail(conv);
    }

    @Transactional
    public ChatDetail appendMessage(String chatId, AppendChatMessageRequest request) {
        if (request.userId() == null || request.userId().isBlank()) {
            throw new IllegalArgumentException("userId required");
        }
        ConversationEntity conv = requireConversation(chatId, request.userId().trim());
        int order = messages.countByConversationId(chatId);
        Instant now = Instant.now();
        messages.save(
                new MessageEntity(
                        chatId, request.role(), request.content().trim(), order, now));
        conv.setUpdatedAt(now);
        if ("New chat".equals(conv.getTitle()) && "user".equals(request.role())) {
            conv.setTitle(titleFromMessage(request.content()));
        }
        conversations.save(conv);
        return toDetail(conv);
    }

    @Transactional
    public ChatDetail update(String chatId, UpdateChatRequest request) {
        if (request.userId() == null || request.userId().isBlank()) {
            throw new IllegalArgumentException("userId required");
        }
        ConversationEntity conv = requireConversation(chatId, request.userId().trim());
        if (request.title() != null && !request.title().isBlank()) {
            conv.setTitle(trimTitle(request.title()));
        }
        if (request.documentId() != null) {
            conv.setDocumentId(request.documentId().isBlank() ? null : request.documentId());
        }
        if (request.documentName() != null) {
            conv.setDocumentName(
                    request.documentName().isBlank() ? null : request.documentName());
        }
        conv.setUpdatedAt(Instant.now());
        conversations.save(conv);
        return toDetail(conv);
    }

    @Transactional
    public void delete(String chatId, String userId) {
        requireConversation(chatId, userId);
        messages.deleteByConversationId(chatId);
        conversations.deleteById(chatId);
    }

    private ConversationEntity requireConversation(String chatId, String userId) {
        return conversations
                .findByIdAndUserId(chatId, userId)
                .orElseThrow(() -> new ChatNotFoundException(chatId));
    }

    private ChatSummary toSummary(ConversationEntity conv) {
        List<MessageEntity> recent =
                messages.findByConversationIdOrderBySortOrderAsc(conv.getId());
        String preview = "";
        if (!recent.isEmpty()) {
            MessageEntity last = recent.get(recent.size() - 1);
            preview = previewFrom(last.getContent());
        }
        return new ChatSummary(
                conv.getId(),
                conv.getTitle(),
                preview,
                conv.getUpdatedAt(),
                recent.size());
    }

    private ChatDetail toDetail(ConversationEntity conv) {
        List<ChatMessage> msgs =
                messages.findByConversationIdOrderBySortOrderAsc(conv.getId()).stream()
                        .map(
                                m ->
                                        new ChatMessage(
                                                m.getRole(), m.getContent(), m.getCreatedAt()))
                        .toList();
        return new ChatDetail(
                conv.getId(),
                conv.getTitle(),
                conv.getSessionId(),
                conv.getDocumentId(),
                conv.getDocumentName(),
                conv.getCreatedAt(),
                conv.getUpdatedAt(),
                msgs);
    }

    private static String titleFromMessage(String content) {
        String oneLine = content.trim().replaceAll("\\s+", " ");
        if (oneLine.isEmpty()) {
            return "New chat";
        }
        return trimTitle(oneLine);
    }

    private static String trimTitle(String text) {
        if (text.length() <= TITLE_MAX) {
            return text;
        }
        return text.substring(0, TITLE_MAX - 1) + "…";
    }

    private static String previewFrom(String content) {
        String oneLine = content.trim().replaceAll("\\s+", " ");
        if (oneLine.length() <= PREVIEW_MAX) {
            return oneLine;
        }
        return oneLine.substring(0, PREVIEW_MAX - 1) + "…";
    }
}
