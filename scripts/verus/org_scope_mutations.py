"""Break the organization scope rules (2026-11-18 bernard-square 403)."""


def cases(source):
    # The outage: every organization claimed Library's platform.
    yield "every_organization_on_library", source.replace(
        "    if same_bytes(home, library) {\n        ScopePlatform::Library\n    } else {\n        ScopePlatform::Other\n    }",
        "    ScopePlatform::Library", 1)
    # The fallback the design removes: an unrecorded organization treated as
    # Library's.
    yield "unrecorded_falls_back_to_library", source.replace(
        "    if !recorded {\n        return ScopePlatform::Unrecorded;\n    }",
        "    if !recorded {\n        return ScopePlatform::Library;\n    }", 1)
    yield "platforms_swapped", source.replace(
        "        ScopePlatform::Library\n    } else {\n        ScopePlatform::Other\n    }",
        "        ScopePlatform::Other\n    } else {\n        ScopePlatform::Library\n    }", 1)
    yield "creation_in_any_tenant", source.replace(
        "    same_bytes(request_tenant, destination)\n",
        "    true\n", 1)
    yield "owner_access_reused_for_another_user", source.replace(
        "    executor_is_user && same_bytes(executor_id, grantee_id)",
        "    executor_is_user", 1)
    yield "owner_access_reused_for_service_accounts", source.replace(
        "    executor_is_user && same_bytes(executor_id, grantee_id)",
        "    same_bytes(executor_id, grantee_id)", 1)
    yield "prefix_counts_as_same_tenant", source.replace(
        "    if left.len() != right.len() {\n        return false;\n    }\n    let mut i = 0;",
        "    let mut i = 0;", 1)
