//! This file contains utility code used throughout the project.

pub mod serde_asset_liability {
    //! Includes [`EitherAssetLiability`], which is a tagged wrapper around
    //! `Either<Asset, Liability>` to make the serialized json nicer to interact with.

    use either::Either;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::player::{Asset, Liability};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "card_type")]
    enum EitherAssetLiability {
        #[serde(rename = "asset")]
        Asset(Asset),
        #[serde(rename = "liability")]
        Liability(Liability),
    }

    impl From<EitherAssetLiability> for Either<Asset, Liability> {
        fn from(w: EitherAssetLiability) -> Self {
            match w {
                EitherAssetLiability::Asset(a) => Either::Left(a),
                EitherAssetLiability::Liability(l) => Either::Right(l),
            }
        }
    }

    impl From<Either<Asset, Liability>> for EitherAssetLiability {
        fn from(e: Either<Asset, Liability>) -> Self {
            match e {
                Either::Left(a) => EitherAssetLiability::Asset(a),
                Either::Right(l) => EitherAssetLiability::Liability(l),
            }
        }
    }

    pub mod value {
        //! Serde module containing an implementation of `serialize` and `deserialize` which can be
        //! used to serialize `Either<Asset, Liability>` to a nicer tagged representation.

        use super::*;

        /// Serializes an `Either<Asset, Liability>` to a nicer tagged representation.
        pub fn serialize<S>(
            value: &Either<Asset, Liability>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            EitherAssetLiability::from(value.clone()).serialize(serializer)
        }

        /// Deserializes the nicer tagged representation back into `Either<Asset, Liability>`
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Either<Asset, Liability>, D::Error>
        where
            D: Deserializer<'de>,
        {
            EitherAssetLiability::deserialize(deserializer).map(Either::from)
        }
    }

    pub mod vec {
        //! Module intended to be used with `#[serde(with)]` containing an implementation of
        //! `serialize` and `deserialize` which can be used to serialize
        //! `Vec<Either<Asset, Liability>>` to a nicer tagged representation.

        use super::*;

        /// Serializes a `Vec<Either<Asset, Liability>>` into a nicer tagged representation
        pub fn serialize<S>(
            value: &[Either<Asset, Liability>],
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mapped = value
                .iter()
                .cloned()
                .map(EitherAssetLiability::from)
                .collect::<Vec<_>>();

            mapped.serialize(serializer)
        }

        /// Deserializes the list of nicer tagged representations back into a
        /// `Vec<Either<Asset, Liability>>`
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
