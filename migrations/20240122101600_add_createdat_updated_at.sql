-- Add migration script here

alter table objects
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;

alter table fields
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;

alter table data
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;

alter table clobs
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;

alter table indexes
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;

alter table relationships
    add column created_at timestamp default current_timestamp not null,
    add column updated_at timestamp default current_timestamp on update current_timestamp not null;
