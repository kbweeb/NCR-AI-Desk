package com.ncr.desk.chat;

public class ChatNotFoundException extends RuntimeException {
    public ChatNotFoundException(String chatId) {
        super("Chat not found: " + chatId);
    }
}
