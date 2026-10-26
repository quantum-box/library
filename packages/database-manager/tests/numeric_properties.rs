//! Live MySQL coverage. Run against an isolated disposable database only.
use database_manager::{
    domain::*, property_definition_rollout::PropertyDefinitionStorageMode,
    property_value_rollout::PropertyValueStorageMode, usecase::*,
};
use std::collections::BTreeMap;
use tachyon_sdk::auth::{Executor, MultiTenancy};
use value_object::TenantId;

#[tokio::test]
#[ignore = "requires isolated NUMERIC_TEST_DATABASE_URL"]
async fn numeric_recipe_round_trips_in_each_storage_mode(
) -> anyhow::Result<()> {
    let dsn = std::env::var("NUMERIC_TEST_DATABASE_URL")?;
    let db = persistence::Db::new(dsn).await;
    database_manager::App::migrate(db.clone()).await?;
    for mode in [
        PropertyValueStorageMode::LegacyOnly,
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyValueStorageMode::DualWriteCanonicalRead,
    ] {
        let app = database_manager::factory_client_with_db(
            db.clone(),
            mode,
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        )?;
        let tenant = TenantId::default();
        let tenancy = MultiTenancy::new_operator(tenant.clone());
        let actor = Executor::SystemUser;
        let create_db = |name| {
            app.create_database().execute(CreateDatabaseInputData {
                executor: &actor,
                multi_tenancy: &tenancy,
                tenant_id: &tenant,
                name,
                database_id: None,
            })
        };
        let lines = create_db("Ingredient lines").await?;
        let recipes = create_db("Recipes").await?;
        let add_property =
            |database_id: DatabaseId, name: String, typ: PropertyType| {
                let port = app.add_property().clone();
                let tenant = tenant.clone();
                let tenancy = tenancy.clone();
                async move {
                    port.execute(AddPropertyInputData {
                        executor: &Executor::SystemUser,
                        multi_tenancy: &tenancy,
                        tenant_id: &tenant,
                        database_id: &database_id,
                        name: &name,
                        display_name: None,
                        property_type: typ,
                    })
                    .await
                }
            };
        let cost = add_property(
            lines.id().clone(),
            "cost".into(),
            PropertyType::Decimal(TypeDecimal::default()),
        )
        .await?;
        let amount = add_property(
            lines.id().clone(),
            "amount".into(),
            PropertyType::Quantity(TypeQuantity::default()),
        )
        .await?;
        let relation = add_property(
            recipes.id().clone(),
            "lines".into(),
            PropertyType::Relation(TypeRelation::new(lines.id().clone())),
        )
        .await?;
        let servings = add_property(
            recipes.id().clone(),
            "servings".into(),
            PropertyType::Integer,
        )
        .await?;
        let rollup_config = TypeRollup {
            relation_property_id: relation.id().to_string(),
            target_property_id: cost.id().to_string(),
            function: RollupFunction::Sum,
            decimal: TypeDecimal::default(),
        };
        let total = add_property(
            recipes.id().clone(),
            "total_cost".into(),
            PropertyType::Rollup(rollup_config.clone()),
        )
        .await?;
        let portion = add_property(
            recipes.id().clone(),
            "per_serving".into(),
            PropertyType::Formula(TypeFormula {
                expression: FormulaExpr::Divide {
                    left: Box::new(FormulaExpr::Property {
                        id: total.id().to_string(),
                    }),
                    right: Box::new(FormulaExpr::Property {
                        id: servings.id().to_string(),
                    }),
                },
                decimal: TypeDecimal::default(),
            }),
        )
        .await?;
        let mut line_ids = Vec::new();
        for (price, quantity, unit) in
            [("125.5", "150.5", "g"), ("249.5", "1.5", "kg")]
        {
            let row = app
                .add_data_usecase()
                .execute(AddDataInputData {
                    executor: &actor,
                    multi_tenancy: &tenancy,
                    tenant_id: &tenant,
                    database_id: lines.id(),
                    name: "Line",
                    property_data: vec![
                        PropertyDataInputData {
                            property_id: cost.id().clone(),
                            value: PropertyValueCommand::Decimal(
                                price.into(),
                            ),
                        },
                        PropertyDataInputData {
                            property_id: amount.id().clone(),
                            value: PropertyValueCommand::Quantity(
                                QuantityValue {
                                    value: quantity.into(),
                                    unit: unit.into(),
                                },
                            ),
                        },
                    ],
                })
                .await?;
            line_ids.push(row.id().clone());
        }
        let recipe = app
            .add_data_usecase()
            .execute(AddDataInputData {
                executor: &actor,
                multi_tenancy: &tenancy,
                tenant_id: &tenant,
                database_id: recipes.id(),
                name: "Recipe fixture",
                property_data: vec![
                    PropertyDataInputData {
                        property_id: relation.id().clone(),
                        value: PropertyValueCommand::Relation(
                            line_ids.clone(),
                        ),
                    },
                    PropertyDataInputData {
                        property_id: servings.id().clone(),
                        value: PropertyValueCommand::Integer(3),
                    },
                ],
            })
            .await?;
        let stored = app
            .get_data_usecase()
            .execute(&GetDataInputData {
                executor: &actor,
                multi_tenancy: &tenancy,
                tenant_id: &tenant,
                database_id: recipes.id(),
                data_id: recipe.id(),
            })
            .await?;
        let props = app
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant.clone(),
                database_id: recipes.id().clone(),
            })
            .await?;
        let load_costs = || async {
            let rows = app
                .data_snapshot()
                .load_ids(&tenant, lines.id(), &line_ids)
                .await?;
            Ok::<_, errors::Error>(
                rows.iter()
                    .map(|row| {
                        match row
                            .get_property_data(cost.id())
                            .and_then(|p| p.value().as_ref())
                        {
                            Some(PropertyDataValue::Decimal(value)) => {
                                ComputedValue::Ready {
                                    value: value.clone(),
                                }
                            }
                            _ => panic!("decimal lost"),
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let result = rollup_config.evaluate(&load_costs().await?);
        assert_eq!(
            result,
            ComputedValue::Ready {
                value: "375".into()
            }
        );
        let output = evaluate_record(
            &stored,
            &props,
            &BTreeMap::from([(total.id().to_string(), result)]),
        )?;
        assert_eq!(
            output.get_property_data(portion.id()).unwrap().value(),
            &Some(PropertyDataValue::Formula(ComputedValue::Ready {
                value: "125".into()
            }))
        );
        let reloaded = app
            .data_snapshot()
            .load_ids(&tenant, lines.id(), &line_ids)
            .await?;
        assert!(
            matches!(reloaded.iter().find(|row|row.id()==&line_ids[1]).unwrap().get_property_data(amount.id()).unwrap().value(),Some(PropertyDataValue::Quantity(q)) if q.unit=="kg" && q.value=="1.5")
        );
        assert!(app
            .data_snapshot()
            .load_ids(&TenantId::default(), lines.id(), &line_ids)
            .await
            .is_err());
        assert!(app
            .update_data_usecase()
            .execute(UpdateDataInputData {
                executor: &actor,
                multi_tenancy: &tenancy,
                tenant_id: &tenant,
                database_id: recipes.id(),
                data_id: recipe.id(),
                name: "Recipe fixture",
                data: vec![PropertyDataInputData {
                    property_id: portion.id().clone(),
                    value: PropertyValueCommand::Clear
                }]
            })
            .await
            .is_err());
        app.update_data_usecase()
            .execute(UpdateDataInputData {
                executor: &actor,
                multi_tenancy: &tenancy,
                tenant_id: &tenant,
                database_id: lines.id(),
                data_id: &line_ids[0],
                name: "Line",
                data: vec![PropertyDataInputData {
                    property_id: cost.id().clone(),
                    value: PropertyValueCommand::Decimal("0.1".into()),
                }],
            })
            .await?;
        assert_eq!(
            rollup_config.evaluate(&load_costs().await?),
            ComputedValue::Ready {
                value: "249.6".into()
            }
        );
    }
    Ok(())
}
