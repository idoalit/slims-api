CREATE TABLE IF NOT EXISTS `mst_subject_type` (
  `subject_type_id` varchar(2) COLLATE utf8_unicode_ci NOT NULL,
  `subject_type_name` varchar(50) COLLATE utf8_unicode_ci NOT NULL,
  `input_date` date NOT NULL,
  `last_update` date DEFAULT NULL,
  PRIMARY KEY (`subject_type_id`),
  UNIQUE KEY `subject_type_name` (`subject_type_name`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8 COLLATE=utf8_unicode_ci;

INSERT INTO `mst_subject_type`
  (`subject_type_id`, `subject_type_name`, `input_date`, `last_update`)
VALUES
  ('t', 'Topik', CURDATE(), CURDATE()),
  ('g', 'Geografis', CURDATE(), CURDATE()),
  ('n', 'Nama', CURDATE(), CURDATE()),
  ('tm', 'Temporal', CURDATE(), CURDATE()),
  ('gr', 'Genre', CURDATE(), CURDATE()),
  ('oc', 'Pekerjaan', CURDATE(), CURDATE())
ON DUPLICATE KEY UPDATE
  `subject_type_name` = VALUES(`subject_type_name`),
  `last_update` = CURDATE();

CREATE TABLE IF NOT EXISTS `mst_subject_level` (
  `subject_level_id` tinyint unsigned NOT NULL AUTO_INCREMENT,
  `subject_level_name` varchar(50) COLLATE utf8_unicode_ci NOT NULL,
  `input_date` date NOT NULL,
  `last_update` date DEFAULT NULL,
  PRIMARY KEY (`subject_level_id`),
  UNIQUE KEY `subject_level_name` (`subject_level_name`)
) ENGINE=MyISAM DEFAULT CHARSET=utf8 COLLATE=utf8_unicode_ci;

INSERT INTO `mst_subject_level`
  (`subject_level_id`, `subject_level_name`, `input_date`, `last_update`)
VALUES
  (1, 'Primer', CURDATE(), CURDATE()),
  (2, 'Tambahan', CURDATE(), CURDATE())
ON DUPLICATE KEY UPDATE
  `subject_level_name` = VALUES(`subject_level_name`),
  `last_update` = CURDATE();

ALTER TABLE `mst_topic`
  MODIFY COLUMN `topic_type` varchar(2) COLLATE utf8_unicode_ci NOT NULL;

ALTER TABLE `biblio_topic`
  MODIFY COLUMN `level` tinyint unsigned NOT NULL DEFAULT 1;
