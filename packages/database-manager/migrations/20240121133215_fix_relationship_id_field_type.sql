-- Add migration script here
drop table relationships;

create table relationships (
    id varchar(31) not null primary key,
    tenant_id VARCHAR(29) NOT NULL,
    object_id VARCHAR(31) NOT NULL,
    field_id VARCHAR(31) NOT NULL,
    relation_id INT UNSIGNED NOT NULL,
    target_object_id VARCHAR(31) NOT NULL,

    CONSTRAINT fk_relationships_data FOREIGN KEY (object_id) REFERENCES data(id),
    INDEX idx_relationships_tenant_id (tenant_id),
    INDEX idx_relationships_object_id (object_id),
    CONSTRAINT fk_relationships_target_object_id
        foreign key (target_object_id) references objects(id),
    CONSTRAINT fk_relationships_field_id
        foreign key (field_id) references fields(id) 
);