-- MySQL and TiDB prohibit CHECK expressions from reading foreign-key columns.
-- RelationDefinition construction and restoration reject a self-inverse that
-- reuses the source Property, so retain that invariant at the domain boundary.
SELECT 1;
