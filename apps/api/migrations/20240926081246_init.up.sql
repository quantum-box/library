-- Add up migration script here
-- repos テーブルの作成
CREATE TABLE `repos` (
  `id` VARCHAR(29) NOT NULL,
  `name` VARCHAR(255) NOT NULL,
  `description` TEXT,
  PRIMARY KEY (`id`)
);

-- repos テーブルの name カラムにインデックスを作成
CREATE INDEX `repos_name_idx` ON `repos` (`name`);

-- policies テーブルの作成
CREATE TABLE `policies` (
  `id` INT NOT NULL AUTO_INCREMENT,
  `user_id` VARCHAR(255) NOT NULL,
  `role` VARCHAR(255) NOT NULL,
  `repo_id` VARCHAR(29) NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`repo_id`) REFERENCES `repos` (`id`) ON DELETE CASCADE
);

-- policies テーブルの repo_id と user_id の複合インデックスを作成
CREATE INDEX `policies_repo_id_user_id_idx` ON `policies` (`repo_id`, `user_id`);

-- databases テーブルの作成
CREATE TABLE `databases` (
  `id` INT NOT NULL AUTO_INCREMENT,
  `database_id` VARCHAR(29) NOT NULL,
  `repo_id` VARCHAR(29) NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`repo_id`) REFERENCES `repos` (`id`) ON DELETE CASCADE
);

