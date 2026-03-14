-- Add down migration script here
-- データベースの削除
DROP TABLE IF EXISTS `databases`;

-- ポリシーの削除
DROP TABLE IF EXISTS `policies`;

-- リポジトリの削除
DROP TABLE IF EXISTS `repos`;
