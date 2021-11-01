use crate::prelude::*;
use ibc_proto::ics23::ProofSpec as ProtoProofSpec;
use ics23::{HashOp, InnerSpec, LeafOp, LengthOp, ProofSpec};

/// An array of proof specifications.
///
/// This type encapsulates different types of proof specifications, mostly predefined, e.g., for
/// Cosmos-SDK.
/// Additionally, this type also aids in the conversion from `ProofSpec` types from crate `ics23`
/// into proof specifications as represented in the `ibc_proto` type; see the
/// `From` trait(s) below.
pub struct ProofSpecs {
    specs: Vec<ProofSpec>,
}

impl ProofSpecs {
    /// Returns the specification for Cosmos-SDK proofs
    pub fn cosmos() -> Self {
        Self {
            specs: vec![
                ics23::iavl_spec(),       // Format of proofs-iavl (iavl merkle proofs)
                ics23::tendermint_spec(), // Format of proofs-tendermint (crypto/ merkle SimpleProof)
            ],
        }
    }

    /// Returns the specification for Cosmos-SDK proofs
    pub fn basecoin() -> Self {
        Self {
            specs: vec![ProofSpec {
                leaf_spec: Some(LeafOp {
                    hash: HashOp::Sha256.into(),
                    prehash_key: HashOp::NoHash.into(),
                    prehash_value: HashOp::NoHash.into(),
                    length: LengthOp::NoPrefix.into(),
                    prefix: [0; 64].to_vec(),
                }),
                inner_spec: Some(InnerSpec {
                    child_order: vec![0, 1, 2],
                    child_size: 32,
                    min_prefix_length: 0,
                    max_prefix_length: 64,
                    empty_child: vec![0, 32],
                    hash: HashOp::Sha256.into(),
                }),
                max_depth: 0,
                min_depth: 0,
            }],
        }
    }
}

/// Converts from the domain type (which is represented as a vector of `ics23::ProofSpec`
/// to the corresponding proto type (vector of `ibc_proto::ProofSpec`).
/// TODO: fix with <https://github.com/informalsystems/ibc-rs/issues/853>
impl From<ProofSpecs> for Vec<ProtoProofSpec> {
    fn from(domain_specs: ProofSpecs) -> Self {
        let mut raw_specs = Vec::new();
        for ds in domain_specs.specs.iter() {
            // Both `ProofSpec` types implement trait `prost::Message`. Convert by encoding, then
            // decoding into the destination type.
            // Safety note: the source and target data structures are identical, hence the
            // encode/decode conversion here should be infallible.
            let mut encoded = Vec::new();
            prost::Message::encode(ds, &mut encoded).unwrap();
            let decoded: ProtoProofSpec = prost::Message::decode(&*encoded).unwrap();
            raw_specs.push(decoded);
        }
        raw_specs
    }
}
