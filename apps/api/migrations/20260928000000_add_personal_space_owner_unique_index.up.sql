CREATE UNIQUE INDEX uniq_org_personal_owner_user_id_platform
ON library.organizations (personal_owner_user_id, platform_id);
