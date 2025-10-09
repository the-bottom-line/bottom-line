pub mod serde_asset_liability {
    use either::Either;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::game::{Asset, Liability};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "card_type")]
    pub enum EitherAssetLiability {
        #[serde(rename = "asset")]
        Left(Asset),
        #[serde(rename = "liability")]
        Right(Liability),
    }

    impl From<EitherAssetLiability> for Either<Asset, Liability> {
        fn from(w: EitherAssetLiability) -> Self {
            match w {
                EitherAssetLiability::Left(a) => Either::Left(a),
                EitherAssetLiability::Right(l) => Either::Right(l),
            }
        }
    }

    impl From<&Either<Asset, Liability>> for EitherAssetLiability {
        fn from(e: &Either<Asset, Liability>) -> Self {
            match e {
                Either::Left(a) => EitherAssetLiability::Left(a.clone()),
                Either::Right(l) => EitherAssetLiability::Right(l.clone()),
            }
        }
    }

    pub mod value {
        use super::*;

        pub fn serialize<S>(
            value: &Either<Asset, Liability>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            EitherAssetLiability::from(value).serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Either<Asset, Liability>, D::Error>
        where
            D: Deserializer<'de>,
        {
            EitherAssetLiability::deserialize(deserializer).map(|e| e.into())
        }
    }

    pub mod vec {
        use super::*;

        pub fn serialize<S>(
            value: &Vec<Either<Asset, Liability>>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mapped = value
                .iter()
                .map(EitherAssetLiability::from)
                .collect::<Vec<_>>();

            mapped.serialize(serializer)
        }

        pub fn deserialize<'de, D>(
            deserializer: D,
        ) -> Result<Vec<Either<Asset, Liability>>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let intermediate = Vec::<EitherAssetLiability>::deserialize(deserializer)?;
            Ok(intermediate.into_iter().map(Either::from).collect())
        }
    }
}
