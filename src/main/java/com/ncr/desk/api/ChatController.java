package com.ncr.desk.api;

import com.ncr.desk.api.dto.ApiError;
import com.ncr.desk.api.dto.AppendChatMessageRequest;
import com.ncr.desk.api.dto.ChatDetail;
import com.ncr.desk.api.dto.ChatSummary;
import com.ncr.desk.api.dto.CreateChatRequest;
import com.ncr.desk.api.dto.UpdateChatRequest;
import com.ncr.desk.chat.ChatNotFoundException;
import com.ncr.desk.chat.ChatService;
import java.util.List;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PatchMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api/chats")
public class ChatController {
    private final ChatService chatService;

    public ChatController(ChatService chatService) {
        this.chatService = chatService;
    }

    @GetMapping
    public List<ChatSummary> list(@RequestParam String userId) {
        if (userId == null || userId.isBlank()) {
            throw new IllegalArgumentException("userId required");
        }
        return chatService.listForUser(userId.trim());
    }

    @GetMapping("/{id}")
    public ChatDetail get(@PathVariable String id, @RequestParam String userId) {
        return chatService.getForUser(id, requireUserId(userId));
    }

    @PostMapping
    public ResponseEntity<ChatDetail> create(@RequestBody CreateChatRequest request) {
        String userId = requireUserId(request.userId());
        ChatDetail created = chatService.create(userId, request.title());
        return ResponseEntity.status(HttpStatus.CREATED).body(created);
    }

    @PostMapping("/{id}/messages")
    public ChatDetail append(
            @PathVariable String id, @RequestBody AppendChatMessageRequest request) {
        if (request.role() == null
                || (!request.role().equals("user") && !request.role().equals("assistant"))) {
            throw new IllegalArgumentException("role must be user or assistant");
        }
        if (request.content() == null || request.content().isBlank()) {
            throw new IllegalArgumentException("content required");
        }
        return chatService.appendMessage(id, new AppendChatMessageRequest(
                requireUserId(request.userId()), request.role(), request.content()));
    }

    @PatchMapping("/{id}")
    public ChatDetail update(@PathVariable String id, @RequestBody UpdateChatRequest request) {
        return chatService.update(id, request);
    }

    @DeleteMapping("/{id}")
    public ResponseEntity<Void> delete(@PathVariable String id, @RequestParam String userId) {
        chatService.delete(id, requireUserId(userId));
        return ResponseEntity.noContent().build();
    }

    @org.springframework.web.bind.annotation.ExceptionHandler(ChatNotFoundException.class)
    public ResponseEntity<ApiError> notFound(ChatNotFoundException ex) {
        return ResponseEntity.status(HttpStatus.NOT_FOUND).body(new ApiError(ex.getMessage()));
    }

    @org.springframework.web.bind.annotation.ExceptionHandler(IllegalArgumentException.class)
    public ResponseEntity<ApiError> badRequest(IllegalArgumentException ex) {
        return ResponseEntity.badRequest().body(new ApiError(ex.getMessage()));
    }

    private static String requireUserId(String userId) {
        if (userId == null || userId.isBlank()) {
            throw new IllegalArgumentException("userId required");
        }
        return userId.trim();
    }
}
