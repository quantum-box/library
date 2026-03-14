# database context

[![hackmd-github-sync-badge](https://hackmd.io/7bD-N3V5RAq7MRGn9pHd9A/badge)](https://hackmd.io/7bD-N3V5RAq7MRGn9pHd9A)


動的スキーマのデータを格納するサービス
カスタム

## usecase

```plantuml
skinparam actorStyle awesome
left to right direction

actor user
actor application

rectangle "database action" {
    usecase "新規DB作成" as create_database
    usecase "プロパティ追加" as add_property
    usecase "プロパティタイプ選択" as choose_property_type
    usecase "データベース定義を見る" as get_database_definition

    usecase "データ追加" as add_data
    usecase "データ更新" as update_data
}

user --> create_database                
user --> add_property
user --> choose_property_type
user --> add_data
user --> update_data

```

## domain

```plantuml

hide methods

rectangle Database {
    entity Database<<root>> {
        DatabaseId
        TenantId
        Name
    }
}

rectangle Property {
    entity Property<<root>> {
        PropertyId
        TenantId
        DatabaseId
        Name
        PropertyType
        IsIndexed
        PropertyNum
        ' Index // user defined index
    }
    
    enum PropertyType {
        String
    }
    Property -> PropertyType
}

rectangle Data {
    entity Data <<root>> {
        DataId
        TenantId
        DatabaseId
        Name
        propertyData 
    }
    
    class PropertyData {
        PropertyId
        Value
    }
    Data "1"-->"1..n" PropertyData
}

note right of Data
    propertyDataは、50propertyまで
    （増やすことは可能）
end note


' 以下は一旦実装しない　追加のリリースとかで
entity Indexes {
    id
    TenantId
    ObjectId
    FieldNum
}

entity Relationships {
    Id
    TenantId
    ObjectId
    RelationId
    TargetObjectId
}

Data }|--|| Database
Property }|--|| Database
Indexes }|--|| Data
Relationships }|--|| Data

```

### Ubiquitous language



| 言語             | eng                 | desc                                       |
| ---------------- | ------------------- | ------------------------------------------ |
| データベース     | database            | RDBでいうtableみたいなもの                 |
| プロパティ       | property            | RDBでいうcolumnにあたる                    |
| プロパティタイプ | property_type       | propertyの制約                             |
| データベース定義 | database_definition | データベースのproperty_type一覧            |
| データ           | data                | データベースの一つ一つの要素、RDBでいうrow　s |


## salesforceを参考にマルチテナントスキーマを構築

https://www.publickey1.jp/blog/09/3_2.html

### Overview

```plantuml
left to right direction

rectangle MetadataTable {
    entity Objects {
        カスタムオブジェクト用のメタデータ
        ---
        Id(ObjectId)
        TenantId
        ObjectName
    }

    entity Fields {
        カスタム項目用のメタデータ
        ---
        Id(FieldId)
        TenantId
        ObjectId
        FieldName
        Datatype
        IsIndexed
        FieldNum
    }   
}

rectangle Datatable{
    entity Data {
        カスタムオブジェクトの構造化データを\n格納する大容量ヒープストレージ
        ---
        Id
        TenantId
        ObjectId
        Name
        Value0(可変長文字列)
        Value1
        Value2
        Value3
        ...
    }

    entity Clobs {
        カスタムオブジェクトの非構造化データを\n格納する大容量ヒープストレージ
    }
}

rectangle PiovotTable {
    entity  Indexes {
        一意でないインデックスを\n格納するピボットテーブル
    }
}


Objects --> Data
Fields --> Data
Clobs -right-> Data

Indexes -up-> Data

```

### ERD

```plantuml

hide method
entity Objects {
    Id(ObjectId)
    TenantId
    ObjectName
}

entity Fields {
    Id(FieldId)
    TenantId
    ObjectId
    FieldName
    Datatype
    IsIndexed
    FieldNum
}  
entity Data {
    Id
    TenantId
    ObjectId
    Name
    Value0(可変長文字列)
    Value1
    Value2
    Value3
    ...
}

entity Clobs {
}

entity Indexes {
    id
    TenantId
    ObjectId
    FieldNum
}

entity Relationships {
    Id
    TenantId
    ObjectId
    RelationId
    TargetObjectId
}

Data }|--|| Objects
Fields }|--|| Objects
Indexes }|--|| Data
Relationships }|--|| Data

```

シェア
https://chat.openai.com/share/59d08e3e-ce0a-4646-93ab-df46c549fbb8

Chat
https://chat.openai.com/c/87a93ac1-886e-49ee-a5a5-1b40f7f88fd0
