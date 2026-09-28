ALTER TABLE library.organizations
ADD COLUMN personal_owner_user_id VARCHAR(255) NULL,
ADD UNIQUE INDEX uniq_org_personal_owner_user_id_platform (
    personal_owner_user_id,
    platform_id
);
