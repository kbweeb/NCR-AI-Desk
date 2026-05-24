package com.ncr.desk.persistence;

import java.util.List;
import org.springframework.data.jpa.repository.JpaRepository;

public interface MessageRepository extends JpaRepository<MessageEntity, Long> {
    List<MessageEntity> findByConversationIdOrderBySortOrderAsc(String conversationId);

    int countByConversationId(String conversationId);

    void deleteByConversationId(String conversationId);
}
