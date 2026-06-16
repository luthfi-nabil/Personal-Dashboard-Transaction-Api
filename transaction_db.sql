CREATE DATABASE IF NOT EXISTS `transaction`
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_uca1400_ai_ci;

USE `transaction`;

CREATE TABLE `app_settings` (
  `app_setting_id` char(36) NOT NULL,
  `app_setting_key` varchar(255) NOT NULL,
  `app_setting_value` varchar(255) NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`app_setting_id`),
  UNIQUE KEY `app_setting_key` (`app_setting_key`),
  UNIQUE KEY `app_setting_unique` (`app_setting_key`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

INSERT INTO `app_settings` (`app_setting_id`, `app_setting_key`, `app_setting_value`, `is_active`) VALUES
('00000000-0000-4000-9000-000000000001', 'TRANSFER_CATEGORY_ID', '00000000-0000-4000-8000-000000000001', 1),
('00000000-0000-4000-9000-000000000002', 'TRANSFER_CATEGORY_NAME', 'Transfer', 1),
('00000000-0000-4000-9000-000000000003', 'RECOUNT_CATEGORY_ID', '00000000-0000-4000-8000-000000000002', 1),
('00000000-0000-4000-9000-000000000004', 'RECOUNT_CATEGORY_NAME', 'Recount', 1),
('00000000-0000-4000-9000-000000000005', 'DEBT_CATEGORY_ID', '00000000-0000-4000-8000-000000000003', 1),
('00000000-0000-4000-9000-000000000006', 'DEBT_CATEGORY_NAME', 'Debt', 1);

CREATE TABLE `debt` (
  `debt_id` char(36) NOT NULL,
  `description` varchar(255) NOT NULL,
  `debt_type` int(11) NOT NULL,
  `total_amount` bigint(20) NOT NULL,
  `debt_transaction_id` char(36) DEFAULT NULL,
  `created_date` datetime NOT NULL,
  `created_by` varchar(255) NOT NULL,
  `is_active` int(11) NOT NULL,
  PRIMARY KEY (`debt_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `debt_paid` (
  `debt_paid_id` char(36) DEFAULT NULL,
  `debt_id` char(36) DEFAULT NULL,
  `debt_paid_transaction_id` char(36) DEFAULT NULL,
  `total_amount` float DEFAULT NULL,
  `created_date` datetime DEFAULT NULL,
  `created_by` varchar(100) DEFAULT NULL,
  `is_active` int(11) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `earning` (
  `earning_id` char(36) NOT NULL,
  `total_amount` double NOT NULL,
  `description` text DEFAULT NULL,
  `earning_category_id` char(36) NOT NULL,
  `earning_category` varchar(255) NOT NULL,
  `source_id` char(255) NOT NULL,
  `source` varchar(255) NOT NULL,
  `created_date` datetime NOT NULL,
  `created_by` text NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`earning_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `earning_category` (
  `earning_category_id` char(36) NOT NULL,
  `earning_category` varchar(255) NOT NULL,
  `created_date` datetime NOT NULL,
  `created_by` varchar(255) NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`earning_category_id`),
  UNIQUE KEY `earning_category` (`earning_category`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `source` (
  `source_id` char(36) NOT NULL,
  `SOURCE` varchar(255) NOT NULL,
  `created_date` datetime NOT NULL,
  `created_by` varchar(255) NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`source_id`),
  UNIQUE KEY `SOURCE` (`SOURCE`, `created_by`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `spending` (
  `spending_id` char(36) NOT NULL,
  `total_amount` double NOT NULL,
  `description` text DEFAULT NULL,
  `spending_category_id` char(36) NOT NULL,
  `spending_category` varchar(255) NOT NULL,
  `source_id` char(255) NOT NULL,
  `source` varchar(255) NOT NULL,
  `created_date` datetime NOT NULL,
  `created_by` text NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`spending_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;

CREATE TABLE `spending_category` (
  `spending_category_id` char(36) NOT NULL,
  `spending_category` varchar(255) NOT NULL,
  `created_date` datetime NOT NULL,
  `created_by` varchar(255) NOT NULL,
  `is_active` int(11) NOT NULL DEFAULT 1,
  PRIMARY KEY (`spending_category_id`),
  UNIQUE KEY `spending_category` (`spending_category`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_uca1400_ai_ci;
