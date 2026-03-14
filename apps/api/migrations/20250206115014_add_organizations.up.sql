CREATE TABLE library.organizations (
    id VARCHAR(29) NOT NULL,
    name VARCHAR(191) NOT NULL,
    username VARCHAR(191) NOT NULL,
    description TEXT,
    PRIMARY KEY (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
