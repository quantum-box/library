-- Add migration script here
alter table relationships
    modify target_object_id varchar(31) not null;

alter table relationships
    add constraint fk_relationships_target_object_id foreign key (target_object_id) references objects(id);

alter table relationships
    add column field_id varchar(31) not null;

alter table relationships
    add constraint fk_relationships_field_id foreign key (field_id) references fields(id);

