-- Add migration script here
alter table relationships
    drop foreign key fk_relationships_data;

alter table relationships
    add constraint fk_relationships_object_id foreign key (object_id) references objects(id);
