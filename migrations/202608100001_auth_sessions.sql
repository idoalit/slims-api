CREATE TABLE IF NOT EXISTS `auth_refresh_sessions` (
  `selector` CHAR(22) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `validator_hash` CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `family_id` CHAR(22) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `user_id` BIGINT NOT NULL,
  `remember_me` BOOLEAN NOT NULL DEFAULT FALSE,
  `expires_at` DATETIME(6) NOT NULL,
  `created_at` DATETIME(6) NOT NULL,
  `last_used_at` DATETIME(6) NULL,
  `revoked_at` DATETIME(6) NULL,
  `replaced_by` CHAR(22) CHARACTER SET ascii COLLATE ascii_bin NULL,
  PRIMARY KEY (`selector`),
  KEY `idx_auth_refresh_family` (`family_id`),
  KEY `idx_auth_refresh_user` (`user_id`),
  KEY `idx_auth_refresh_expiry` (`expires_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
