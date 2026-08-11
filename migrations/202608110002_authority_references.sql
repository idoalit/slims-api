CREATE TABLE IF NOT EXISTS `mst_authority_type` (
  `authority_type_id` char(1) COLLATE utf8_unicode_ci NOT NULL,
  `authority_type_name` varchar(50) COLLATE utf8_unicode_ci NOT NULL,
  `input_date` date NOT NULL,
  `last_update` date DEFAULT NULL,
  PRIMARY KEY (`authority_type_id`),
  UNIQUE KEY `authority_type_name` (`authority_type_name`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8 COLLATE=utf8_unicode_ci;

INSERT INTO `mst_authority_type`
  (`authority_type_id`, `authority_type_name`, `input_date`, `last_update`)
VALUES
  ('p', 'Nama perorangan', CURDATE(), CURDATE()),
  ('o', 'Badan organisasi', CURDATE(), CURDATE()),
  ('c', 'Konferensi', CURDATE(), CURDATE())
ON DUPLICATE KEY UPDATE
  `authority_type_name` = VALUES(`authority_type_name`),
  `last_update` = CURDATE();

CREATE TABLE IF NOT EXISTS `mst_authority_level` (
  `authority_level_id` tinyint unsigned NOT NULL AUTO_INCREMENT,
  `authority_level_name` varchar(50) COLLATE utf8_unicode_ci NOT NULL,
  `input_date` date NOT NULL,
  `last_update` date DEFAULT NULL,
  PRIMARY KEY (`authority_level_id`),
  UNIQUE KEY `authority_level_name` (`authority_level_name`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8 COLLATE=utf8_unicode_ci;

INSERT INTO `mst_authority_level`
  (`authority_level_id`, `authority_level_name`, `input_date`, `last_update`)
VALUES
  (1, 'Pengarang utama', CURDATE(), CURDATE()),
  (2, 'Pengarang tambahan', CURDATE(), CURDATE()),
  (3, 'Editor', CURDATE(), CURDATE()),
  (4, 'Penerjemah', CURDATE(), CURDATE()),
  (5, 'Direktur', CURDATE(), CURDATE()),
  (6, 'Produser', CURDATE(), CURDATE()),
  (7, 'Komposer', CURDATE(), CURDATE()),
  (8, 'Ilustrator', CURDATE(), CURDATE()),
  (9, 'Kreator', CURDATE(), CURDATE()),
  (10, 'Kontributor', CURDATE(), CURDATE())
ON DUPLICATE KEY UPDATE
  `authority_level_name` = VALUES(`authority_level_name`),
  `last_update` = CURDATE();

ALTER TABLE `mst_author`
  MODIFY COLUMN `authority_type` char(1) COLLATE utf8_unicode_ci NOT NULL DEFAULT 'p';

ALTER TABLE `biblio_author`
  MODIFY COLUMN `level` tinyint unsigned NOT NULL DEFAULT 1;
