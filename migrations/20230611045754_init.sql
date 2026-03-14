-- Add migration script here

-- objects table
CREATE TABLE objects (
    id VARCHAR(29) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(29) NOT NULL,
    object_name VARCHAR(255) NOT NULL,
    INDEX idx_objects_tenant_id (tenant_id)
);

-- fields table
CREATE TABLE fields (
    id VARCHAR(31) NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(29) NOT NULL,
    object_id VARCHAR(29) NOT NULL,
    field_name VARCHAR(255) NOT NULL,
    datatype VARCHAR(255) NOT NULL,
    is_indexed BOOLEAN NOT NULL,
    field_num INT UNSIGNED NOT NULL,
    CONSTRAINT fk_fields_objects FOREIGN KEY (object_id) REFERENCES objects(id),
    INDEX idx_fields_tenant_id (tenant_id),
    INDEX idx_fields_object_id (object_id)
);

-- data table
CREATE TABLE data (
    id VARCHAR(31) PRIMARY KEY,
    tenant_id VARCHAR(29) not null,
    object_id VARCHAR(29) not null,
    name VARCHAR(255) not null,
    value0 TEXT,
    value1 TEXT,
    value2 TEXT,
    value3 TEXT,
    value4 TEXT,
    value5 TEXT,
    value6 TEXT,
    value7 TEXT,
    value8 TEXT,
    value9 TEXT,
    value10 TEXT,
    value11 TEXT,
    value12 TEXT,
    value13 TEXT,
    value14 TEXT,
    value15 TEXT,
    value16 TEXT,
    value17 TEXT,
    value18 TEXT,
    value19 TEXT,
    value20 TEXT,
    value21 TEXT,
    value22 TEXT,
    value23 TEXT,
    value24 TEXT,
    value25 TEXT,
    value26 TEXT,
    value27 TEXT,
    value28 TEXT,
    value29 TEXT,
    value30 TEXT,
    value31 TEXT,
    value32 TEXT,
    value33 TEXT,
    value34 TEXT,
    value35 TEXT,
    value36 TEXT,
    value37 TEXT,
    value38 TEXT,
    value39 TEXT,
    value40 TEXT,
    value41 TEXT,
    value42 TEXT,
    value43 TEXT,
    value44 TEXT,
    value45 TEXT,
    value46 TEXT,
    value47 TEXT,
    value48 TEXT,
    value49 TEXT,
    value50 TEXT,
    CONSTRAINT fk_data_objects FOREIGN KEY (object_id) REFERENCES objects(id),
    INDEX idx_data_tenant_id (tenant_id),
    INDEX idx_data_object_id (object_id)
);

-- clobs table
CREATE TABLE clobs (
    -- Assuming that you forgot to include the fields in your description, 
    -- here I'm creating an id field
    id INT UNSIGNED NOT NULL PRIMARY KEY
);

-- indexes table
CREATE TABLE indexes (
    id INT UNSIGNED NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(29) NOT NULL,
    object_id VARCHAR(31) NOT NULL,
    field_num INT UNSIGNED NOT NULL,
    CONSTRAINT fk_indexes_data FOREIGN KEY (object_id) REFERENCES data(id),
    INDEX idx_indexes_tenant_id (tenant_id),
    INDEX idx_indexes_object_id (object_id)
);

-- relationships table
CREATE TABLE relationships (
    id INT UNSIGNED NOT NULL PRIMARY KEY,
    tenant_id VARCHAR(29) NOT NULL,
    object_id VARCHAR(31) NOT NULL,
    relation_id INT UNSIGNED NOT NULL,
    target_object_id INT UNSIGNED NOT NULL,
    CONSTRAINT fk_relationships_data FOREIGN KEY (object_id) REFERENCES data(id),
    INDEX idx_relationships_tenant_id (tenant_id),
    INDEX idx_relationships_object_id (object_id)
);



