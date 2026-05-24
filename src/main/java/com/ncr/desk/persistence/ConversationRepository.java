package com.ncr.desk.persistence;

import java.util.List;
import java.util.Optional;
import org.springframework.data.jpa.repository.JpaRepository;

public interface ConversationRepository extends JpaRepository<ConversationEntity, String> {
    List<ConversationEntity> findByUserIdOrderByUpdatedAtDesc(String userId);

    Optional<ConversationEntity> findByIdAndUserId(String id, String userId);
}
