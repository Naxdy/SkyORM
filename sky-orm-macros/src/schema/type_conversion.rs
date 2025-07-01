use proc_macro2::TokenStream;
use quote::quote;
use sqlparser::ast::DataType;

pub fn sql_to_rust_type(sql_type: &DataType) -> TokenStream {
    match sql_type {
        DataType::Table(_) => todo!(),
        DataType::TinyText
        | DataType::MediumText
        | DataType::LongText
        | DataType::String(_)
        | DataType::FixedString(_)
        | DataType::Text
        | DataType::Uuid
        | DataType::Nvarchar(_)
        | DataType::Varchar(_)
        | DataType::CharVarying(_)
        | DataType::CharacterVarying(_)
        | DataType::Char(_)
        | DataType::Character(_) => quote! {
            ::std::string::String
        },
        DataType::CharacterLargeObject(_) => todo!(),
        DataType::CharLargeObject(_) => todo!(),
        DataType::Clob(_) => todo!(),
        DataType::Binary(_) => todo!(),
        DataType::Varbinary(_) => todo!(),
        DataType::Blob(_) => todo!(),
        DataType::TinyBlob => todo!(),
        DataType::MediumBlob => todo!(),
        DataType::LongBlob => todo!(),
        DataType::Bytes(_) => todo!(),
        DataType::Decimal(_)
        | DataType::BigNumeric(_)
        | DataType::Numeric(_)
        | DataType::BigDecimal(_) => quote! {
            f64
        },
        DataType::TinyInt(_) | DataType::Int2(_) | DataType::SmallInt(_) | DataType::Int8(_) => {
            quote! {
                i8
            }
        }
        DataType::MediumInt(_) | DataType::Int4(_) | DataType::Int16 => quote! {
            i16
        },
        DataType::Int(_) | DataType::Int32 => quote! {
            i32
        },
        DataType::Int64 => quote! {
            i64
        },
        DataType::Int256 => todo!(),
        DataType::TinyIntUnsigned(_)
        | DataType::UTinyInt
        | DataType::Int2Unsigned(_)
        | DataType::SmallIntUnsigned(_)
        | DataType::USmallInt
        | DataType::UInt8 => quote! {
            u8
        },
        DataType::MediumIntUnsigned(_) | DataType::Int4Unsigned(_) | DataType::UInt16 => quote! {
            u16
        },
        DataType::IntUnsigned(_) | DataType::IntegerUnsigned(_) | DataType::UInt32 => quote! {
            u32
        },
        DataType::UInt64 => quote! {
            u64
        },
        DataType::UInt256 => todo!(),
        DataType::Int128 | DataType::HugeInt | DataType::BigInt(_) => quote! {
            i128
        },
        DataType::UHugeInt
        | DataType::UInt128
        | DataType::BigIntUnsigned(_)
        | DataType::UBigInt => quote! {
            u128
        },
        DataType::Int8Unsigned(_) => quote! {
            i64
        },
        DataType::Integer(_) | DataType::SignedInteger | DataType::Signed => quote! {
            i32
        },
        DataType::UnsignedInteger | DataType::Unsigned => quote! {
            u32
        },
        DataType::Dec(_)
        | DataType::Float(_)
        | DataType::Float4
        | DataType::Float32
        | DataType::Real => quote! {
            f32
        },
        DataType::Float64 | DataType::Float8 | DataType::DoublePrecision | DataType::Double(_) => {
            quote! {
                f64
            }
        }
        DataType::Bool | DataType::Boolean => quote! {
            bool
        },
        DataType::Date32 | DataType::Date => quote! {
            ::chrono::NaiveDate
        },
        DataType::Time(_, _) => quote! {
            ::chrono::NaiveTime
        },
        DataType::TimestampNtz | DataType::Datetime64(_, _) | DataType::Datetime(_) => quote! {
            ::chrono::NaiveDateTime
        },
        DataType::Timestamp(_, timezone_info) => match timezone_info {
            sqlparser::ast::TimezoneInfo::WithoutTimeZone | sqlparser::ast::TimezoneInfo::None => {
                quote! {
                    ::chrono::NaiveDateTime
                }
            }
            sqlparser::ast::TimezoneInfo::Tz | sqlparser::ast::TimezoneInfo::WithTimeZone => {
                quote! {
                    ::chrono::DateTime<::chrono::FixedOffset>
                }
            }
        },
        DataType::Interval => todo!(),
        DataType::JSONB | DataType::JSON => quote! {
            ::sky_orm::sqlx::types::JsonRawValue
        },
        DataType::Regclass => todo!(),
        DataType::Bytea => todo!(),
        DataType::Bit(_) => todo!(),
        DataType::BitVarying(_) => todo!(),
        DataType::VarBit(_) => todo!(),
        DataType::Custom(_, _) => todo!(),
        DataType::Array(_) => todo!(),
        DataType::Map(_, _) => todo!(),
        DataType::Tuple(_) => todo!(),
        DataType::Nested(_) => todo!(),
        DataType::Enum(_, _) => todo!(),
        DataType::Set(_) => todo!(),
        DataType::Struct(_, _) => todo!(),
        DataType::Union(_) => todo!(),
        DataType::Nullable(data_type) => {
            let inner_type = sql_to_rust_type(data_type);

            quote! {
                ::std::option::Option<#inner_type>
            }
        }
        DataType::LowCardinality(_) => todo!(),
        DataType::Unspecified => todo!(),
        DataType::Trigger => todo!(),
        DataType::AnyType => todo!(),
        DataType::GeometricType(_) => todo!(),
    }
}
