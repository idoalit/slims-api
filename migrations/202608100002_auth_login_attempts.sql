CREATE TABLE IF NOT EXISTS `auth_login_attempts` (
  `username_hash` CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `attempts` INT UNSIGNED NOT NULL,
  `window_started_at` DATETIME(6) NOT NULL,
  `locked_until` DATETIME(6) NULL,
  PRIMARY KEY (`username_hash`),
  KEY `idx_auth_login_attempts_lock` (`locked_until`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
