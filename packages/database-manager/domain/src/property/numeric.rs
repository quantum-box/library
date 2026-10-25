//! Exact numeric contracts. Wire values are decimal strings, never JSON floats.
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TypeDecimal {
    pub precision: u32,
    pub scale: u32,
}
impl Default for TypeDecimal {
    fn default() -> Self {
        Self {
            precision: 28,
            scale: 12,
        }
    }
}
impl TypeDecimal {
    pub fn validate(&self) -> errors::Result<()> {
        if self.precision == 0
            || self.precision > 28
            || self.scale > self.precision
        {
            return Err(errors::Error::invalid(
                "decimal requires 1 <= precision <= 28 and scale <= precision",
            ));
        }
        Ok(())
    }
    pub fn parse(&self, text: &str) -> errors::Result<Decimal> {
        self.validate()?;
        // Restrict both work and syntax before passing to the decimal parser.
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let mut parts = unsigned.split('.');
        let whole = parts.next().unwrap_or("");
        let fraction = parts.next();
        if text.len() > 64
            || whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || fraction.is_some_and(|s| {
                s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit())
            })
            || parts.next().is_some()
        {
            return Err(errors::Error::invalid(
                "decimal must be a plain decimal string (no exponent)",
            ));
        }
        let fraction = fraction.unwrap_or("");
        let integer_digits = whole.trim_start_matches('0').len() as u32;
        if fraction.len() as u32 > self.scale
            || integer_digits > self.precision - self.scale
        {
            return Err(errors::Error::invalid(
                "decimal exceeds configured precision or scale",
            ));
        }
        Decimal::from_str(text).map_err(errors::Error::invalid)
    }
    pub fn normalize(&self, text: &str) -> errors::Result<String> {
        Ok(self.parse(text)?.normalize().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityValue {
    pub value: String,
    pub unit: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TypeQuantity {
    pub decimal: TypeDecimal,
    pub allowed_units: Vec<String>,
    pub default_unit: String,
    pub unit_version: u16,
}
impl Default for TypeQuantity {
    fn default() -> Self {
        Self {
            decimal: TypeDecimal::default(),
            allowed_units: vec!["g".into(), "kg".into()],
            default_unit: "g".into(),
            unit_version: 1,
        }
    }
}
/// Stable v1 IDs, dimensions and exact factors relative to g, mL, count.
fn unit(id: &str) -> errors::Result<(&'static str, Decimal)> {
    match id {
        "g" => Ok(("mass", Decimal::ONE)),
        "kg" => Ok(("mass", Decimal::from(1000))),
        "mL" => Ok(("volume", Decimal::ONE)),
        "L" => Ok(("volume", Decimal::from(1000))),
        "count" => Ok(("count", Decimal::ONE)),
        _ => Err(errors::Error::invalid("unknown unit ID")),
    }
}
impl TypeQuantity {
    pub fn validate(&self) -> errors::Result<()> {
        self.decimal.validate()?;
        if self.unit_version != 1
            || self.allowed_units.is_empty()
            || !self.allowed_units.contains(&self.default_unit)
        {
            return Err(errors::Error::invalid(
                "invalid quantity units or unit version",
            ));
        }
        let dimension = unit(&self.default_unit)?.0;
        let mut seen = BTreeSet::new();
        for id in &self.allowed_units {
            if unit(id)?.0 != dimension || !seen.insert(id) {
                return Err(errors::Error::invalid(
                    "quantity units must be unique and of one dimension",
                ));
            }
        }
        Ok(())
    }
    pub fn normalize(
        &self,
        value: &QuantityValue,
    ) -> errors::Result<QuantityValue> {
        self.validate()?;
        if !self.allowed_units.contains(&value.unit) {
            return Err(errors::Error::invalid("unit is not allowed"));
        }
        Ok(QuantityValue {
            value: self.decimal.normalize(&value.value)?,
            unit: value.unit.clone(),
        })
    }
    pub fn convert(
        &self,
        value: &QuantityValue,
        target: &str,
    ) -> errors::Result<QuantityValue> {
        let value = self.normalize(value)?;
        if !self.allowed_units.iter().any(|id| id == target) {
            return Err(errors::Error::invalid(
                "target unit is not allowed",
            ));
        }
        let (from_dim, from_factor) = unit(&value.unit)?;
        let (to_dim, to_factor) = unit(target)?;
        if from_dim != to_dim {
            return Err(errors::Error::invalid(
                "cannot convert different dimensions",
            ));
        }
        // Convert exactly before enforcing the destination scale. Decimal's
        // fixed-width division can underflow tiny values to zero.
        let amount = bigdecimal::BigDecimal::from_str(&value.value)
            .map_err(errors::Error::invalid)?;
        let from_factor =
            bigdecimal::BigDecimal::from_str(&from_factor.to_string())
                .map_err(errors::Error::invalid)?;
        let to_factor =
            bigdecimal::BigDecimal::from_str(&to_factor.to_string())
                .map_err(errors::Error::invalid)?;
        let result = amount * from_factor / to_factor;
        self.normalize(&QuantityValue {
            value: result.normalized().to_plain_string(),
            unit: target.into(),
        })
    }
    pub fn compare(
        &self,
        left: &QuantityValue,
        right: &QuantityValue,
    ) -> errors::Result<std::cmp::Ordering> {
        let left = self.normalize(left)?;
        let right = self.normalize(right)?;
        let numeric = |value: &QuantityValue| -> errors::Result<bigdecimal::BigDecimal> {
            let amount = bigdecimal::BigDecimal::from_str(&value.value).map_err(errors::Error::invalid)?;
            let factor = if matches!(value.unit.as_str(), "kg" | "L") { 1000 } else { 1 };
            Ok(amount * bigdecimal::BigDecimal::from(factor))
        };
        Ok(numeric(&left)?.cmp(&numeric(&right)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum FormulaExpr {
    Literal {
        value: String,
    },
    Property {
        id: String,
    },
    Add {
        left: Box<Self>,
        right: Box<Self>,
    },
    Subtract {
        left: Box<Self>,
        right: Box<Self>,
    },
    Multiply {
        left: Box<Self>,
        right: Box<Self>,
    },
    Divide {
        left: Box<Self>,
        right: Box<Self>,
    },
    Equal {
        left: Box<Self>,
        right: Box<Self>,
    },
    LessThan {
        left: Box<Self>,
        right: Box<Self>,
    },
    If {
        condition: Box<Self>,
        then: Box<Self>,
        otherwise: Box<Self>,
    },
    Round {
        value: Box<Self>,
        scale: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeFormula {
    pub expression: FormulaExpr,
    #[serde(default)]
    pub decimal: TypeDecimal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollupFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeRollup {
    pub relation_property_id: String,
    pub target_property_id: String,
    pub function: RollupFunction,
    #[serde(default)]
    pub decimal: TypeDecimal,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComputedValue {
    Ready { value: String },
    Null,
    Pending,
    Error { code: String },
}
fn parse_numeric(text: &str) -> errors::Result<Decimal> {
    let scale = text
        .split_once('.')
        .map_or(0, |(_, fractional)| fractional.len() as u32);
    TypeDecimal {
        precision: 28,
        scale,
    }
    .parse(text)
}

impl FormulaExpr {
    pub fn dependencies(&self) -> errors::Result<BTreeSet<String>> {
        let mut ids = BTreeSet::new();
        let mut budget = 256;
        self.visit(&mut ids, 0, &mut budget)?;
        Ok(ids)
    }
    fn visit(
        &self,
        ids: &mut BTreeSet<String>,
        depth: usize,
        budget: &mut usize,
    ) -> errors::Result<()> {
        if depth > 32 || *budget == 0 {
            return Err(errors::Error::invalid(
                "formula exceeds complexity limit",
            ));
        }
        *budget -= 1;
        match self {
            Self::Literal { value } => {
                parse_numeric(value)?;
            }
            Self::Property { id } => {
                if id.is_empty() {
                    return Err(errors::Error::invalid(
                        "empty property reference",
                    ));
                }
                ids.insert(id.clone());
            }
            Self::Add { left, right }
            | Self::Subtract { left, right }
            | Self::Multiply { left, right }
            | Self::Divide { left, right }
            | Self::Equal { left, right }
            | Self::LessThan { left, right } => {
                left.visit(ids, depth + 1, budget)?;
                right.visit(ids, depth + 1, budget)?;
            }
            Self::If {
                condition,
                then,
                otherwise,
            } => {
                condition.visit(ids, depth + 1, budget)?;
                then.visit(ids, depth + 1, budget)?;
                otherwise.visit(ids, depth + 1, budget)?;
            }
            Self::Round { value, scale } => {
                if *scale > 28 {
                    return Err(errors::Error::invalid(
                        "round scale exceeds 28",
                    ));
                }
                value.visit(ids, depth + 1, budget)?;
            }
        }
        Ok(())
    }
    fn eval(
        &self,
        read: &impl Fn(&str) -> ComputedValue,
        allow_rounding: bool,
    ) -> Result<Option<Decimal>, String> {
        let parse = |text: &str| {
            parse_numeric(text).map_err(|_| "invalid_decimal".into())
        };
        let both = |a: &Self,
                    b: &Self|
         -> Result<Option<(Decimal, Decimal)>, String> {
            Ok(
                match (
                    a.eval(read, allow_rounding)?,
                    b.eval(read, allow_rounding)?,
                ) {
                    (Some(a), Some(b)) => Some((a, b)),
                    _ => None,
                },
            )
        };
        let checked = |v: Option<Decimal>| {
            v.map(Some).ok_or_else(|| "overflow".into())
        };
        let exact = |value: Option<Decimal>,
                     expected: bigdecimal::BigDecimal|
         -> Result<Option<Decimal>, String> {
            let value = value.ok_or_else(|| "overflow".to_string())?;
            if !allow_rounding
                && bigdecimal::BigDecimal::from_str(&value.to_string())
                    .map_err(|_| "overflow".to_string())?
                    != expected
            {
                return Err("inexact_result".into());
            }
            Ok(Some(value))
        };
        let big = |value: Decimal| {
            bigdecimal::BigDecimal::from_str(&value.to_string())
                .expect("finite decimal")
        };
        match self {
            Self::Literal { value } => parse(value).map(Some),
            Self::Property { id } => match read(id) {
                ComputedValue::Ready { value } => parse(&value).map(Some),
                ComputedValue::Null => Ok(None),
                ComputedValue::Pending => Err("pending".into()),
                ComputedValue::Error { code } => Err(code),
            },
            Self::If {
                condition,
                then,
                otherwise,
            } => match condition.eval(read, allow_rounding)? {
                Some(v) if v != Decimal::ZERO => {
                    then.eval(read, allow_rounding)
                }
                Some(_) => otherwise.eval(read, allow_rounding),
                None => Ok(None),
            },
            Self::Round { value, scale } => {
                Ok(value.eval(read, true)?.map(|v| {
                    v.round_dp_with_strategy(
                        *scale,
                        RoundingStrategy::MidpointAwayFromZero,
                    )
                }))
            }
            _ => {
                let (a, b) = match self {
                    Self::Add { left, right }
                    | Self::Subtract { left, right }
                    | Self::Multiply { left, right }
                    | Self::Divide { left, right }
                    | Self::Equal { left, right }
                    | Self::LessThan { left, right } => {
                        match both(left, right)? {
                            Some(v) => v,
                            None => return Ok(None),
                        }
                    }
                    _ => unreachable!(),
                };
                match self {
                    Self::Add { .. } => {
                        exact(a.checked_add(b), big(a) + big(b))
                    }
                    Self::Subtract { .. } => {
                        exact(a.checked_sub(b), big(a) - big(b))
                    }
                    Self::Multiply { .. } => {
                        exact(a.checked_mul(b), big(a) * big(b))
                    }
                    Self::Divide { .. } => {
                        if b == Decimal::ZERO {
                            Err("division_by_zero".into())
                        } else {
                            let result = a
                                .checked_div(b)
                                .ok_or_else(|| "overflow".to_string())?;
                            if !allow_rounding
                                && big(result) * big(b) != big(a)
                            {
                                return Err("inexact_division".into());
                            }
                            checked(Some(result))
                        }
                    }
                    Self::Equal { .. } => {
                        Ok(Some(Decimal::from(i32::from(a == b))))
                    }
                    Self::LessThan { .. } => {
                        Ok(Some(Decimal::from(i32::from(a < b))))
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
impl TypeFormula {
    pub fn validate(&self) -> errors::Result<()> {
        self.decimal.validate()?;
        self.expression.dependencies()?;
        Ok(())
    }
    pub fn evaluate(
        &self,
        read: impl Fn(&str) -> ComputedValue,
    ) -> ComputedValue {
        if self.validate().is_err() {
            return ComputedValue::Error {
                code: "invalid_expression".into(),
            };
        }
        match self.expression.eval(&read, false) {
            Ok(Some(value)) => computed_decimal(&self.decimal, value),
            Ok(None) => ComputedValue::Null,
            Err(code) if code == "pending" => ComputedValue::Pending,
            Err(code) => ComputedValue::Error { code },
        }
    }
}
fn computed_decimal(config: &TypeDecimal, value: Decimal) -> ComputedValue {
    match config.normalize(&value.normalize().to_string()) {
        Ok(value) => ComputedValue::Ready { value },
        Err(_) => ComputedValue::Error {
            code: "precision_or_scale".into(),
        },
    }
}
impl TypeRollup {
    pub fn validate(&self) -> errors::Result<()> {
        self.decimal.validate()?;
        if self.relation_property_id.is_empty()
            || self.target_property_id.is_empty()
        {
            return Err(errors::Error::invalid(
                "rollup requires property IDs",
            ));
        }
        Ok(())
    }
    /// Callers must supply a complete, authorized, deduplicated target set.
    /// A missing or forbidden record is Error/Pending, never silently omitted.
    pub fn evaluate(&self, values: &[ComputedValue]) -> ComputedValue {
        if self.validate().is_err() {
            return ComputedValue::Error {
                code: "invalid_rollup".into(),
            };
        }
        let mut numbers = Vec::new();
        for value in values {
            match value {
                ComputedValue::Ready { value } => {
                    match self.decimal.parse(value) {
                        Ok(v) => numbers.push(v),
                        Err(_) => {
                            return ComputedValue::Error {
                                code: "invalid_decimal".into(),
                            };
                        }
                    }
                }
                ComputedValue::Null => {}
                other => return other.clone(),
            }
        }
        if self.function == RollupFunction::Count {
            return computed_decimal(
                &self.decimal,
                Decimal::from(values.len() as u64),
            );
        }
        if numbers.is_empty() {
            return ComputedValue::Null;
        }
        let value = match self.function {
            RollupFunction::Min => *numbers.iter().min().unwrap(),
            RollupFunction::Max => *numbers.iter().max().unwrap(),
            _ => {
                let Some(sum) = numbers
                    .iter()
                    .try_fold(Decimal::ZERO, |a, b| a.checked_add(*b))
                else {
                    return ComputedValue::Error {
                        code: "overflow".into(),
                    };
                };
                let exact_sum = numbers.iter().fold(
                    bigdecimal::BigDecimal::from(0),
                    |sum, value| {
                        sum + bigdecimal::BigDecimal::from_str(
                            &value.to_string(),
                        )
                        .expect("decimal")
                    },
                );
                if bigdecimal::BigDecimal::from_str(&sum.to_string())
                    .expect("decimal")
                    != exact_sum
                {
                    return ComputedValue::Error {
                        code: "inexact_result".into(),
                    };
                }
                if self.function == RollupFunction::Avg {
                    match sum
                        .checked_div(Decimal::from(numbers.len() as u64))
                    {
                        Some(v) => {
                            if bigdecimal::BigDecimal::from_str(
                                &v.to_string(),
                            )
                            .expect("decimal")
                                * bigdecimal::BigDecimal::from(
                                    numbers.len() as u64,
                                )
                                != exact_sum
                            {
                                return ComputedValue::Error {
                                    code: "inexact_division".into(),
                                };
                            }
                            v
                        }
                        None => {
                            return ComputedValue::Error {
                                code: "overflow".into(),
                            };
                        }
                    }
                } else {
                    sum
                }
            }
        };
        computed_decimal(&self.decimal, value)
    }
}

/// Validate stable-ID dependencies on the entire proposed schema.
pub fn validate_computed_schema(
    properties: &[crate::Property],
) -> errors::Result<()> {
    use crate::PropertyType;
    let by_id: std::collections::BTreeMap<_, _> =
        properties.iter().map(|p| (p.id().to_string(), p)).collect();
    fn visit(
        id: &str,
        by_id: &std::collections::BTreeMap<String, &crate::Property>,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
    ) -> errors::Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if !active.insert(id.to_string()) {
            return Err(errors::Error::invalid(
                "cyclic computed property dependency",
            ));
        }
        let property = by_id.get(id).ok_or_else(|| {
            errors::Error::invalid("unknown property reference")
        })?;
        match property.property_type() {
            PropertyType::Formula(config) => {
                config.validate()?;
                for dep in config.expression.dependencies()? {
                    let target = by_id.get(&dep).ok_or_else(|| {
                        errors::Error::invalid(
                            "unknown formula property reference",
                        )
                    })?;
                    if !matches!(
                        target.property_type(),
                        PropertyType::Decimal(_)
                            | PropertyType::Integer
                            | PropertyType::Formula(_)
                            | PropertyType::Rollup(_)
                    ) {
                        return Err(errors::Error::invalid(
                            "formula dependency is not numeric",
                        ));
                    }
                    visit(&dep, by_id, active, done)?;
                }
            }
            PropertyType::Rollup(config) => {
                config.validate()?;
                let relation = by_id
                    .get(&config.relation_property_id)
                    .ok_or_else(|| {
                        errors::Error::invalid("unknown rollup relation")
                    })?;
                if !matches!(
                    relation.property_type(),
                    PropertyType::Relation(_)
                ) {
                    return Err(errors::Error::invalid(
                        "rollup requires a relation property",
                    ));
                }
            }
            _ => {}
        }
        active.remove(id);
        done.insert(id.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for id in by_id.keys() {
        visit(id, &by_id, &mut BTreeSet::new(), &mut done)?;
    }
    Ok(())
}

/// Compute a record from its current inputs and already-authorized rollups.
/// Nothing is persisted: every read evaluates against the current dependencies.
pub fn evaluate_record(
    record: &crate::Data,
    properties: &[crate::Property],
    rollups: &std::collections::BTreeMap<String, ComputedValue>,
) -> errors::Result<crate::Data> {
    use crate::{PropertyDataValue as V, PropertyType as T};
    fn resolve(
        id: &str,
        record: &crate::Data,
        properties: &[crate::Property],
        rollups: &std::collections::BTreeMap<String, ComputedValue>,
        active: &mut BTreeSet<String>,
        cache: &mut std::collections::BTreeMap<String, ComputedValue>,
    ) -> ComputedValue {
        if let Some(v) = cache.get(id) {
            return v.clone();
        }
        if !active.insert(id.into()) {
            return ComputedValue::Error {
                code: "cycle".into(),
            };
        }
        let result = match properties.iter().find(|p| p.id().as_str() == id)
        {
            None => ComputedValue::Error {
                code: "missing_dependency".into(),
            },
            Some(p) => match p.property_type() {
                T::Formula(config) => {
                    let values: std::collections::BTreeMap<_, _> = config
                        .expression
                        .dependencies()
                        .unwrap_or_default()
                        .iter()
                        .map(|dep| {
                            (
                                dep.clone(),
                                resolve(
                                    dep, record, properties, rollups,
                                    active, cache,
                                ),
                            )
                        })
                        .collect();
                    config.evaluate(|dep| {
                        values.get(dep).cloned().unwrap_or(
                            ComputedValue::Error {
                                code: "missing_dependency".into(),
                            },
                        )
                    })
                }
                T::Rollup(_) => rollups
                    .get(id)
                    .cloned()
                    .unwrap_or(ComputedValue::Pending),
                T::Integer | T::Decimal(_) => match record
                    .get_property_data(p.id())
                    .and_then(|v| v.value().as_ref())
                {
                    Some(V::Decimal(value)) => ComputedValue::Ready {
                        value: value.clone(),
                    },
                    Some(V::Integer(value)) => ComputedValue::Ready {
                        value: value.to_string(),
                    },
                    None => ComputedValue::Null,
                    _ => ComputedValue::Error {
                        code: "type_mismatch".into(),
                    },
                },
                _ => ComputedValue::Error {
                    code: "type_mismatch".into(),
                },
            },
        };
        active.remove(id);
        cache.insert(id.into(), result.clone());
        result
    }
    let mut cache = std::collections::BTreeMap::new();
    let mut output = record.clone();
    for p in properties {
        if matches!(p.property_type(), T::Formula(_) | T::Rollup(_)) {
            let result = resolve(
                p.id().as_str(),
                record,
                properties,
                rollups,
                &mut BTreeSet::new(),
                &mut cache,
            );
            let value = match p.property_type() {
                T::Formula(_) => V::Formula(result),
                _ => V::Rollup(result),
            };
            let pd = crate::PropertyData::from_computed(p, value)?;
            output = output.with_derived_value(pd);
        }
    }
    Ok(output)
}

impl Default for TypeFormula {
    fn default() -> Self {
        Self {
            expression: FormulaExpr::Literal { value: "0".into() },
            decimal: TypeDecimal::default(),
        }
    }
}
impl Default for TypeRollup {
    fn default() -> Self {
        Self {
            relation_property_id: String::new(),
            target_property_id: String::new(),
            function: RollupFunction::Sum,
            decimal: TypeDecimal::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BUILTIN_PROPERTY_TYPE_REGISTRY, LegacyPropertyValueCodec, Property,
        PropertyData, PropertyDataValue, PropertyType,
        PropertyValueCommand,
    };
    use serde_json::json;
    fn ready(value: &str) -> ComputedValue {
        ComputedValue::Ready {
            value: value.into(),
        }
    }
    fn literal(value: &str) -> Box<FormulaExpr> {
        Box::new(FormulaExpr::Literal {
            value: value.into(),
        })
    }
    fn property(typ: PropertyType) -> Property {
        Property::new(
            &Default::default(),
            &Default::default(),
            &Default::default(),
            "value",
            &typ,
            false,
            0,
        )
    }
    #[test]
    fn decimal_validates_and_round_trips_without_floats() {
        let p = property(PropertyType::Decimal(TypeDecimal::default()));
        for text in [
            "150.5",
            "0.1",
            "0",
            "-12.345",
            "9999999999999999.123456789012",
        ] {
            let value =
                PropertyDataValue::new(text, p.property_type()).unwrap();
            let config = p.property_type().canonical_config();
            let envelope = BUILTIN_PROPERTY_TYPE_REGISTRY
                .encode_envelope(&config, &value)
                .unwrap();
            assert_eq!(envelope.raw_value, json!(text));
            assert_eq!(
                LegacyPropertyValueCodec::encode(&value, p.property_type())
                    .unwrap(),
                text
            );
            assert!(PropertyData::from_envelope(&p, envelope).is_ok());
        }
        for invalid in [
            "NaN",
            "1e2",
            "+1",
            "1.",
            ".1",
            " 1",
            "10000000000000000",
            "0.1234567890123",
        ] {
            assert!(
                PropertyDataValue::new(invalid, p.property_type()).is_err(),
                "{invalid}"
            );
        }
        let c = TypeDecimal::default();
        assert!(c.parse("2").unwrap() < c.parse("10").unwrap());
        assert_eq!(c.normalize("-000.000").unwrap(), "0");
        assert!(
            TypeDecimal {
                precision: 0,
                scale: 0
            }
            .validate()
            .is_err()
        );
        assert!(
            TypeDecimal {
                precision: 28,
                scale: 29
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn quantities_preserve_units_and_reject_incompatible_dimensions() {
        let config = TypeQuantity::default();
        let q = QuantityValue {
            value: "1.5".into(),
            unit: "kg".into(),
        };
        assert_eq!(
            config.convert(&q, "g").unwrap(),
            QuantityValue {
                value: "1500".into(),
                unit: "g".into()
            }
        );
        assert_eq!(
            config
                .compare(
                    &q,
                    &QuantityValue {
                        value: "1500".into(),
                        unit: "g".into()
                    }
                )
                .unwrap(),
            std::cmp::Ordering::Equal
        );
        assert!(config.convert(&q, "mL").is_err());
        assert!(
            config
                .normalize(&QuantityValue {
                    value: "1".into(),
                    unit: "lb".into()
                })
                .is_err()
        );
        let volume = TypeQuantity {
            allowed_units: vec!["mL".into(), "L".into()],
            default_unit: "mL".into(),
            ..config.clone()
        };
        assert_eq!(
            volume
                .convert(
                    &QuantityValue {
                        value: "1".into(),
                        unit: "L".into()
                    },
                    "mL"
                )
                .unwrap()
                .value,
            "1000"
        );
        let mixed = TypeQuantity {
            allowed_units: vec!["kg".into(), "mL".into()],
            default_unit: "kg".into(),
            ..config
        };
        assert!(mixed.validate().is_err());
    }
    #[test]
    fn formulas_are_exact_bounded_and_have_explicit_failure_states() {
        let formula = |expression| TypeFormula {
            expression,
            decimal: TypeDecimal::default(),
        };
        let add = formula(FormulaExpr::Add {
            left: literal("0.1"),
            right: literal("0.2"),
        });
        assert_eq!(add.evaluate(|_| unreachable!()), ready("0.3"));
        assert_eq!(
            formula(FormulaExpr::Divide {
                left: literal("1"),
                right: literal("0")
            })
            .evaluate(|_| unreachable!()),
            ComputedValue::Error {
                code: "division_by_zero".into()
            }
        );
        let reference =
            formula(FormulaExpr::Property { id: "cost".into() });
        assert_eq!(
            reference.evaluate(|_| ComputedValue::Null),
            ComputedValue::Null
        );
        assert_eq!(
            reference.evaluate(|_| ComputedValue::Pending),
            ComputedValue::Pending
        );
        assert_eq!(
            reference.evaluate(|_| ComputedValue::Error {
                code: "access_denied".into()
            }),
            ComputedValue::Error {
                code: "access_denied".into()
            }
        );
        let branch = formula(FormulaExpr::If {
            condition: literal("0"),
            then: Box::new(FormulaExpr::Divide {
                left: literal("1"),
                right: literal("0"),
            }),
            otherwise: literal("125"),
        });
        assert_eq!(branch.evaluate(|_| unreachable!()), ready("125"));
        assert!(
            serde_json::from_value::<FormulaExpr>(
                json!({"op":"eval","code":"fetch()"})
            )
            .is_err()
        );
        let mut deep = FormulaExpr::Literal { value: "0".into() };
        for _ in 0..40 {
            deep = FormulaExpr::Round {
                value: Box::new(deep),
                scale: 2,
            };
        }
        assert!(deep.dependencies().is_err());
    }
    #[test]
    fn rollups_distinguish_empty_null_zero_and_errors() {
        let mut rollup = TypeRollup {
            relation_property_id: "lines".into(),
            target_property_id: "cost".into(),
            function: RollupFunction::Sum,
            decimal: TypeDecimal::default(),
        };
        let values = [ready("0.1"), ComputedValue::Null, ready("0.2")];
        assert_eq!(rollup.evaluate(&values), ready("0.3"));
        assert_eq!(rollup.evaluate(&[]), ComputedValue::Null);
        assert_eq!(rollup.evaluate(&[ready("0")]), ready("0"));
        rollup.function = RollupFunction::Count;
        assert_eq!(rollup.evaluate(&values), ready("3"));
        assert_eq!(rollup.evaluate(&[]), ready("0"));
        rollup.function = RollupFunction::Avg;
        assert_eq!(rollup.evaluate(&values), ready("0.15"));
        rollup.function = RollupFunction::Min;
        assert_eq!(rollup.evaluate(&values), ready("0.1"));
        rollup.function = RollupFunction::Max;
        assert_eq!(rollup.evaluate(&values), ready("0.2"));
        assert_eq!(
            rollup.evaluate(&[ComputedValue::Pending]),
            ComputedValue::Pending
        );
        assert_eq!(
            rollup.evaluate(&[ComputedValue::Error {
                code: "access_denied".into()
            }]),
            ComputedValue::Error {
                code: "access_denied".into()
            }
        );
    }
    #[test]
    fn formula_graph_and_computed_writes_fail_closed() {
        let p = property(PropertyType::Formula(TypeFormula::default()));
        assert!(PropertyValueCommand::Clear.into_value(&p).is_err());
        assert!(PropertyData::new(&p, "{}".into()).is_err());
        assert!(
            PropertyValueCommand::from_legacy_command_text(&p, "").is_err()
        );
        let mut recursive = TypeFormula::default();
        recursive.expression = FormulaExpr::Property {
            id: p.id().to_string(),
        };
        let cycle = Property::new(
            p.id(),
            p.tenant_id(),
            p.database_id(),
            "cycle",
            &PropertyType::Formula(recursive),
            false,
            0,
        );
        assert!(validate_computed_schema(&[cycle]).is_err());
    }
    #[test]
    fn recipe_fixture_recomputes_cost_per_serving_and_rate() {
        let total = property(PropertyType::Decimal(TypeDecimal::default()));
        let servings = property(PropertyType::Integer);
        let portion = property(PropertyType::Formula(TypeFormula {
            expression: FormulaExpr::Divide {
                left: Box::new(FormulaExpr::Property {
                    id: total.id().to_string(),
                }),
                right: Box::new(FormulaExpr::Property {
                    id: servings.id().to_string(),
                }),
            },
            decimal: TypeDecimal::default(),
        }));
        let rate = property(PropertyType::Formula(TypeFormula {
            expression: FormulaExpr::Multiply {
                left: Box::new(FormulaExpr::Divide {
                    left: Box::new(FormulaExpr::Property {
                        id: portion.id().to_string(),
                    }),
                    right: literal("1000"),
                }),
                right: literal("100"),
            },
            decimal: TypeDecimal::default(),
        }));
        let properties = vec![
            total.clone(),
            servings.clone(),
            portion.clone(),
            rate.clone(),
        ];
        validate_computed_schema(&properties).unwrap();
        let mut record = crate::Data::new(
            &Default::default(),
            total.tenant_id(),
            total.database_id(),
            "recipe",
            vec![
                PropertyData::new(&total, "375".into()).unwrap(),
                PropertyData::new(&servings, "3".into()).unwrap(),
            ],
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
        .unwrap();
        let first =
            evaluate_record(&record, &properties, &Default::default())
                .unwrap();
        assert_eq!(
            first.get_property_data(rate.id()).unwrap().value(),
            &Some(PropertyDataValue::Formula(ready("12.5")))
        );
        assert_eq!(first.updated_at(), record.updated_at());
        record
            .update_property_data(
                &PropertyData::new(&total, "750".into()).unwrap(),
            )
            .unwrap();
        let next =
            evaluate_record(&record, &properties, &Default::default())
                .unwrap();
        assert_eq!(
            next.get_property_data(rate.id()).unwrap().value(),
            &Some(PropertyDataValue::Formula(ready("25")))
        );
    }
}

#[cfg(test)]
mod precision_tests {
    use super::*;
    #[test]
    fn division_requires_explicit_rounding_for_nonterminating_results() {
        let quotient = FormulaExpr::Divide {
            left: Box::new(FormulaExpr::Literal { value: "1".into() }),
            right: Box::new(FormulaExpr::Literal { value: "3".into() }),
        };
        let exact = TypeFormula {
            expression: quotient.clone(),
            decimal: TypeDecimal::default(),
        };
        assert_eq!(
            exact.evaluate(|_| unreachable!()),
            ComputedValue::Error {
                code: "inexact_division".into()
            }
        );
        let rounded = TypeFormula {
            expression: FormulaExpr::Round {
                value: Box::new(quotient),
                scale: 2,
            },
            decimal: TypeDecimal::default(),
        };
        assert_eq!(
            rounded.evaluate(|_| unreachable!()),
            ComputedValue::Ready {
                value: "0.33".into()
            }
        );
    }
    #[test]
    fn average_never_implicitly_rounds() {
        let rollup = TypeRollup {
            relation_property_id: "r".into(),
            target_property_id: "p".into(),
            function: RollupFunction::Avg,
            decimal: TypeDecimal::default(),
        };
        let values = ["1", "0", "0"]
            .map(|v| ComputedValue::Ready { value: v.into() });
        assert_eq!(
            rollup.evaluate(&values),
            ComputedValue::Error {
                code: "inexact_division".into()
            }
        );
    }
}

#[cfg(test)]
mod quantity_precision_tests {
    use super::*;
    #[test]
    fn conversion_never_underflows_to_zero() {
        let quantity = TypeQuantity {
            decimal: TypeDecimal {
                precision: 28,
                scale: 28,
            },
            ..TypeQuantity::default()
        };
        let tiny = QuantityValue {
            value: "0.0000000000000000000000000001".into(),
            unit: "g".into(),
        };
        assert!(quantity.convert(&tiny, "kg").is_err());
        let representable = QuantityValue {
            value: "0.000000001".into(),
            unit: "g".into(),
        };
        assert_eq!(
            quantity.convert(&representable, "kg").unwrap().value,
            "0.000000000001"
        );
    }
}
