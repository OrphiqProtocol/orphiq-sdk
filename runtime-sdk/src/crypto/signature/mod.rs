//! Cryptographic signatures.
use std::convert::TryFrom;

use digest::{typenum::Unsigned as _, Digest as _};
use rand_core::{CryptoRng, RngCore};
use thiserror::Error;

use crate::core::common::crypto::signature::{PublicKey as CorePublicKey, Signer as CoreSigner};

pub mod context;
mod digests;
pub mod ed25519;
pub mod secp256k1;
pub mod secp256r1;
pub mod secp384r1;
pub mod sr25519;

/// A specific combination of signature and hash.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
pub enum SignatureType {
    #[cbor(rename = "ed25519_oasis")]
    Ed25519_Oasis,
    #[cbor(rename = "ed25519_pure")]
    Ed25519_Pure,
    #[cbor(rename = "ed25519_prehashed_sha512")]
    Ed25519_PrehashedSha512,
    #[cbor(rename = "secp256k1_oasis")]
    Secp256k1_Oasis,
    #[cbor(rename = "secp256k1_prehashed_keccak256")]
    Secp256k1_PrehashedKeccak256,
    #[cbor(rename = "secp256k1_prehashed_sha256")]
    Secp256k1_PrehashedSha256,
    #[cbor(rename = "sr25519_pure")]
    Sr25519_Pure,
    #[cbor(rename = "secp256r1_prehashed_sha256")]
    Secp256r1_PrehashedSha256,
    #[cbor(rename = "secp384r1_prehashed_sha384")]
    Secp384r1_PrehashedSha384,
}

impl SignatureType {
    pub fn as_int(&self) -> u8 {
        match self {
            Self::Ed25519_Oasis => 0,
            Self::Ed25519_Pure => 1,
            Self::Ed25519_PrehashedSha512 => 2,
            Self::Secp256k1_Oasis => 3,
            Self::Secp256k1_PrehashedKeccak256 => 4,
            Self::Secp256k1_PrehashedSha256 => 5,
            Self::Sr25519_Pure => 6,
            Self::Secp256r1_PrehashedSha256 => 7,
            Self::Secp384r1_PrehashedSha384 => 8,
        }
    }

    pub fn is_prehashed(&self) -> bool {
        matches!(
            self,
            Self::Ed25519_PrehashedSha512
                | Self::Secp256k1_PrehashedKeccak256
                | Self::Secp256k1_PrehashedSha256
                | Self::Secp256r1_PrehashedSha256
                | Self::Secp384r1_PrehashedSha384
        )
    }

    pub fn is_ed25519_variant(&self) -> bool {
        matches!(
            self,
            Self::Ed25519_Oasis | Self::Ed25519_Pure | Self::Ed25519_PrehashedSha512
        )
    }

    pub fn is_secp256k1_variant(&self) -> bool {
        matches!(
            self,
            Self::Secp256k1_Oasis
                | Self::Secp256k1_PrehashedKeccak256
                | Self::Secp256k1_PrehashedSha256
        )
    }

    pub fn is_secp256r1_variant(&self) -> bool {
        matches!(self, Self::Secp256r1_PrehashedSha256)
    }

    pub fn is_secp384r1_variant(&self) -> bool {
        matches!(self, Self::Secp384r1_PrehashedSha384)
    }

    pub fn is_sr25519_variant(&self) -> bool {
        matches!(self, Self::Sr25519_Pure)
    }
}

impl TryFrom<u8> for SignatureType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Ed25519_Oasis),
            1 => Ok(Self::Ed25519_Pure),
            2 => Ok(Self::Ed25519_PrehashedSha512),
            3 => Ok(Self::Secp256k1_Oasis),
            4 => Ok(Self::Secp256k1_PrehashedKeccak256),
            5 => Ok(Self::Secp256k1_PrehashedSha256),
            6 => Ok(Self::Sr25519_Pure),
            7 => Ok(Self::Secp256r1_PrehashedSha256),
            8 => Ok(Self::Secp384r1_PrehashedSha384),
            _ => Err(Error::InvalidArgument),
        }
    }
}

/// A public key used for signing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, cbor::Encode, cbor::Decode)]
pub enum PublicKey {
    #[cbor(rename = "ed25519")]
    Ed25519(ed25519::PublicKey),

    #[cbor(rename = "secp256k1")]
    Secp256k1(secp256k1::PublicKey),

    #[cbor(rename = "secp256r1")]
    Secp256r1(secp256r1::PublicKey),

    #[cbor(rename = "secp384r1")]
    Secp384r1(secp384r1::PublicKey),

    #[cbor(rename = "sr25519")]
    Sr25519(sr25519::PublicKey),
}

/// Error.
#[derive(Error, Debug)]
pub enum Error {
    #[error("malformed public key")]
    MalformedPublicKey,
    #[error("malformed private key")]
    MalformedPrivateKey,
    #[error("malformed signature")]
    MalformedSignature,
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("invalid argument")]
    InvalidArgument,
    #[error("invalid digest length")]
    InvalidDigestLength,
    #[error("other signing error")]
    SigningError,
}

impl PublicKey {
    /// Return the key type as string.
    pub fn key_type(&self) -> &str {
        match self {
            Self::Ed25519(_) => "ed25519",
            Self::Secp256k1(_) => "secp256k1",
            Self::Secp256r1(_) => "secp256r1",
            Self::Secp384r1(_) => "secp384r1",
            Self::Sr25519(_) => "sr25519",
        }
    }

    /// Return a byte representation of this public key.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PublicKey::Ed25519(pk) => pk.as_bytes(),
            PublicKey::Secp256k1(pk) => pk.as_bytes(),
            PublicKey::Secp256r1(pk) => pk.as_bytes(),
            PublicKey::Secp384r1(pk) => pk.as_bytes(),
            PublicKey::Sr25519(pk) => pk.as_bytes(),
        }
    }

    /// Construct a public key from a slice of bytes.
    pub fn from_bytes(sig_type: SignatureType, bytes: &[u8]) -> Result<Self, Error> {
        match sig_type {
            SignatureType::Ed25519_Oasis
            | SignatureType::Ed25519_Pure
            | SignatureType::Ed25519_PrehashedSha512 => {
                Ok(Self::Ed25519(ed25519::PublicKey::from_bytes(bytes)?))
            }
            SignatureType::Secp256k1_Oasis
            | SignatureType::Secp256k1_PrehashedKeccak256
            | SignatureType::Secp256k1_PrehashedSha256 => {
                Ok(Self::Secp256k1(secp256k1::PublicKey::from_bytes(bytes)?))
            }
            SignatureType::Secp256r1_PrehashedSha256 => {
                Ok(Self::Secp256r1(secp256r1::PublicKey::from_bytes(bytes)?))
            }
            SignatureType::Secp384r1_PrehashedSha384 => {
                Ok(Self::Secp384r1(secp384r1::PublicKey::from_bytes(bytes)?))
            }
            SignatureType::Sr25519_Pure => {
                Ok(Self::Sr25519(sr25519::PublicKey::from_bytes(bytes)?))
            }
        }
    }

    /// Verify a signature.
    pub fn verify(
        &self,
        context: &[u8],
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), Error> {
        match self {
            PublicKey::Ed25519(pk) => pk.verify(context, message, signature),
            PublicKey::Secp256k1(pk) => pk.verify(context, message, signature),
            PublicKey::Secp256r1(pk) => pk.verify(context, message, signature),
            PublicKey::Secp384r1(pk) => pk.verify(context, message, signature),
            PublicKey::Sr25519(pk) => pk.verify(context, message, signature),
        }
    }

    /// Verify signature raw using the underlying method, without the domain
    /// separation schema.
    pub fn verify_raw(&self, message: &[u8], signature: &Signature) -> Result<(), Error> {
        match self {
            PublicKey::Ed25519(pk) => pk.verify_raw(message, signature),
            PublicKey::Secp256k1(pk) => pk.verify_raw(message, signature),
            PublicKey::Secp256r1(pk) => pk.verify_raw(message, signature),
            PublicKey::Secp384r1(pk) => pk.verify_raw(message, signature),
            PublicKey::Sr25519(_) => Err(Error::InvalidArgument),
        }
    }

    /// Verify the signature of a message.
    pub fn verify_by_type(
        &self,
        signature_type: SignatureType,
        context_or_hash: &[u8],
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), Error> {
        match self {
            Self::Ed25519(pk) => match signature_type {
                SignatureType::Ed25519_Oasis => pk.verify(context_or_hash, message, signature),
                SignatureType::Ed25519_Pure => pk.verify_raw(message, signature),
                SignatureType::Ed25519_PrehashedSha512 => {
                    if context_or_hash.len()
                        != <sha2::Sha512 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha512>::new_precomputed(context_or_hash);
                    pk.verify_digest(digest, signature)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp256k1(pk) => match signature_type {
                SignatureType::Secp256k1_Oasis => pk.verify(context_or_hash, message, signature),
                SignatureType::Secp256k1_PrehashedKeccak256 => {
                    if context_or_hash.len()
                        != <sha3::Keccak256 as sha3::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    // Use SHA-256 for RFC6979 even if Keccak256 was used for the message.
                    let digest = digests::DummyDigest::<k256::sha2::Sha256>::new_precomputed(
                        context_or_hash,
                    );
                    pk.verify_digest(digest, signature)
                }
                SignatureType::Secp256k1_PrehashedSha256 => {
                    if context_or_hash.len()
                        != <sha2::Sha256 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest = digests::DummyDigest::<k256::sha2::Sha256>::new_precomputed(
                        context_or_hash,
                    );
                    pk.verify_digest(digest, signature)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp256r1(pk) => match signature_type {
                SignatureType::Secp256r1_PrehashedSha256 => {
                    if context_or_hash.len()
                        != <sha2::Sha256 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha256>::new_precomputed(context_or_hash);
                    pk.verify_digest(digest, signature)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp384r1(pk) => match signature_type {
                SignatureType::Secp384r1_PrehashedSha384 => {
                    if context_or_hash.len()
                        != <sha2::Sha384 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha384>::new_precomputed(context_or_hash);
                    pk.verify_digest(digest, signature)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Sr25519(pk) => match signature_type {
                SignatureType::Sr25519_Pure => pk.verify_raw(context_or_hash, message, signature),
                _ => Err(Error::InvalidArgument),
            },
        }
    }

    /// Verify a batch of signatures of the same message.
    pub fn verify_batch_multisig(
        context: &[u8],
        message: &[u8],
        public_keys: &[PublicKey],
        signatures: &[Signature],
    ) -> Result<(), Error> {
        if public_keys.len() != signatures.len() {
            return Err(Error::InvalidArgument);
        }

        // TODO: Use actual batch verification.
        for (pk, sig) in public_keys.iter().zip(signatures.iter()) {
            pk.verify(context, message, sig)?;
        }
        Ok(())
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq<CorePublicKey> for PublicKey {
    fn eq(&self, other: &CorePublicKey) -> bool {
        match self {
            PublicKey::Ed25519(pk) => pk.as_bytes() == other.as_ref(),
            _ => false,
        }
    }
}

impl TryFrom<PublicKey> for CorePublicKey {
    type Error = &'static str;

    fn try_from(pk: PublicKey) -> Result<Self, Self::Error> {
        match pk {
            PublicKey::Ed25519(pk) => Ok(pk.into()),
            _ => Err("not an Ed25519 public key"),
        }
    }
}

impl From<CorePublicKey> for PublicKey {
    fn from(pk: CorePublicKey) -> Self {
        Self::Ed25519(pk.into())
    }
}

/// Variable-length opaque signature.
#[derive(Clone, Debug, Default, PartialEq, Eq, cbor::Encode, cbor::Decode)]
#[cbor(transparent)]
pub struct Signature(Vec<u8>);

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for Signature {
    fn from(v: Vec<u8>) -> Signature {
        Signature(v)
    }
}

impl From<Signature> for Vec<u8> {
    fn from(s: Signature) -> Vec<u8> {
        s.0
    }
}

/// Common trait for memory signers.
pub trait Signer: Send + Sync {
    /// Create a new random signer.
    fn random(rng: &mut (impl RngCore + CryptoRng)) -> Result<Self, Error>
    where
        Self: Sized;

    /// Create a new signer from the given seed.
    fn new_from_seed(seed: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;

    /// Recreate signer from a byte serialization.
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;

    /// Serialize the signer into bytes.
    fn to_bytes(&self) -> Vec<u8>;

    /// Return the public key counterpart to the signer's secret key.
    fn public_key(&self) -> PublicKey;

    /// Generate a signature over the context and message.
    fn sign(&self, context: &[u8], message: &[u8]) -> Result<Signature, Error>;

    /// Generate a signature over the message.
    fn sign_raw(&self, message: &[u8]) -> Result<Signature, Error>;
}

impl<T: Signer + ?Sized> Signer for std::sync::Arc<T> {
    fn random(_rng: &mut (impl RngCore + CryptoRng)) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn new_from_seed(_seed: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn from_bytes(_bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn to_bytes(&self) -> Vec<u8> {
        T::to_bytes(self)
    }

    fn public_key(&self) -> PublicKey {
        T::public_key(self)
    }

    fn sign(&self, context: &[u8], message: &[u8]) -> Result<Signature, Error> {
        T::sign(self, context, message)
    }

    fn sign_raw(&self, message: &[u8]) -> Result<Signature, Error> {
        T::sign_raw(self, message)
    }
}

impl<T: CoreSigner> Signer for &T {
    fn random(_rng: &mut (impl RngCore + CryptoRng)) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn new_from_seed(_seed: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn from_bytes(_bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.public().into())
    }

    fn sign(&self, context: &[u8], message: &[u8]) -> Result<Signature, Error> {
        let raw_sig = CoreSigner::sign(*self, context, message).map_err(|_| Error::SigningError)?;
        Ok(Signature(raw_sig.as_ref().into()))
    }

    fn sign_raw(&self, _message: &[u8]) -> Result<Signature, Error> {
        Err(Error::InvalidArgument)
    }
}

impl Signer for crate::core::identity::Identity {
    fn random(_rng: &mut (impl RngCore + CryptoRng)) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn new_from_seed(_seed: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn from_bytes(_bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::InvalidArgument)
    }

    fn to_bytes(&self) -> Vec<u8> {
        vec![]
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.public().into())
    }

    fn sign(&self, context: &[u8], message: &[u8]) -> Result<Signature, Error> {
        let raw_sig = CoreSigner::sign(self, context, message).map_err(|_| Error::SigningError)?;
        Ok(Signature(raw_sig.as_ref().into()))
    }

    fn sign_raw(&self, _message: &[u8]) -> Result<Signature, Error> {
        Err(Error::InvalidArgument)
    }
}

/// A memory-backed signer.
pub enum MemorySigner {
    Ed25519(ed25519::MemorySigner),
    Secp256k1(secp256k1::MemorySigner),
    Secp256r1(secp256r1::MemorySigner),
    Secp384r1(secp384r1::MemorySigner),
    Sr25519(sr25519::MemorySigner),
}

impl MemorySigner {
    /// Create a new memory signer from a seed.
    pub fn new_from_seed(sig_type: SignatureType, seed: &[u8]) -> Result<Self, Error> {
        if sig_type.is_ed25519_variant() {
            Ok(Self::Ed25519(ed25519::MemorySigner::new_from_seed(seed)?))
        } else if sig_type.is_secp256k1_variant() {
            Ok(Self::Secp256k1(secp256k1::MemorySigner::new_from_seed(
                seed,
            )?))
        } else if sig_type.is_secp256r1_variant() {
            Ok(Self::Secp256r1(secp256r1::MemorySigner::new_from_seed(
                seed,
            )?))
        } else if sig_type.is_secp384r1_variant() {
            Ok(Self::Secp384r1(secp384r1::MemorySigner::new_from_seed(
                seed,
            )?))
        } else if sig_type.is_sr25519_variant() {
            Ok(Self::Sr25519(sr25519::MemorySigner::new_from_seed(seed)?))
        } else {
            Err(Error::InvalidArgument)
        }
    }

    /// Create a new signer for testing purposes.
    pub fn new_test(sig_type: SignatureType, name: &str) -> Self {
        if sig_type.is_secp384r1_variant() {
            Self::new_from_seed(sig_type, &sha2::Sha384::digest(name)).unwrap()
        } else {
            Self::new_from_seed(sig_type, &sha2::Sha512_256::digest(name)).unwrap()
        }
    }

    /// Reconstruct the signer from its byte representation.
    pub fn from_bytes(sig_type: SignatureType, bytes: &[u8]) -> Result<Self, Error> {
        if sig_type.is_ed25519_variant() {
            Ok(Self::Ed25519(ed25519::MemorySigner::from_bytes(bytes)?))
        } else if sig_type.is_secp256k1_variant() {
            Ok(Self::Secp256k1(secp256k1::MemorySigner::from_bytes(bytes)?))
        } else if sig_type.is_secp256r1_variant() {
            Ok(Self::Secp256r1(secp256r1::MemorySigner::from_bytes(bytes)?))
        } else if sig_type.is_secp384r1_variant() {
            Ok(Self::Secp384r1(secp384r1::MemorySigner::from_bytes(bytes)?))
        } else if sig_type.is_sr25519_variant() {
            Ok(Self::Sr25519(sr25519::MemorySigner::from_bytes(bytes)?))
        } else {
            Err(Error::InvalidArgument)
        }
    }

    /// Return a byte representation of the signer.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519(signer) => signer.to_bytes(),
            Self::Secp256k1(signer) => signer.to_bytes(),
            Self::Secp256r1(signer) => signer.to_bytes(),
            Self::Secp384r1(signer) => signer.to_bytes(),
            Self::Sr25519(signer) => signer.to_bytes(),
        }
    }

    /// Public key corresponding to the signer.
    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Ed25519(signer) => signer.public_key(),
            Self::Secp256k1(signer) => signer.public_key(),
            Self::Secp256r1(signer) => signer.public_key(),
            Self::Secp384r1(signer) => signer.public_key(),
            Self::Sr25519(signer) => signer.public_key(),
        }
    }

    /// Generate a signature with the private key over the context and message.
    pub fn sign(&self, context: &[u8], message: &[u8]) -> Result<Signature, Error> {
        match self {
            Self::Ed25519(signer) => signer.sign(context, message),
            Self::Secp256k1(signer) => signer.sign(context, message),
            Self::Secp256r1(signer) => signer.sign(context, message),
            Self::Secp384r1(signer) => signer.sign(context, message),
            Self::Sr25519(signer) => signer.sign(context, message),
        }
    }

    /// Generate a signature with the private key over the message.
    pub fn sign_raw(&self, message: &[u8]) -> Result<Signature, Error> {
        match self {
            Self::Ed25519(signer) => signer.sign_raw(message),
            Self::Secp256k1(signer) => signer.sign_raw(message),
            Self::Secp256r1(signer) => signer.sign_raw(message),
            Self::Secp384r1(signer) => signer.sign_raw(message),
            Self::Sr25519(signer) => signer.sign_raw(message),
        }
    }

    /// Generate a signature for the specified message and optional context.
    pub fn sign_by_type(
        &self,
        signature_type: SignatureType,
        context_or_hash: &[u8],
        message: &[u8],
    ) -> Result<Signature, Error> {
        match self {
            Self::Ed25519(signer) => match signature_type {
                SignatureType::Ed25519_Oasis => signer.sign(context_or_hash, message),
                SignatureType::Ed25519_Pure => signer.sign_raw(message),
                SignatureType::Ed25519_PrehashedSha512 => {
                    if context_or_hash.len()
                        != <sha2::Sha512 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha512>::new_precomputed(context_or_hash);
                    signer.sign_digest(digest)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp256k1(signer) => match signature_type {
                SignatureType::Secp256k1_Oasis => signer.sign(context_or_hash, message),
                SignatureType::Secp256k1_PrehashedKeccak256 => {
                    if context_or_hash.len()
                        != <sha3::Keccak256 as sha3::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    // Use SHA-256 for RFC6979 even if Keccak256 was used for the message.
                    let digest = digests::DummyDigest::<k256::sha2::Sha256>::new_precomputed(
                        context_or_hash,
                    );
                    signer.sign_digest(digest)
                }
                SignatureType::Secp256k1_PrehashedSha256 => {
                    if context_or_hash.len()
                        != <sha2::Sha256 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest = digests::DummyDigest::<k256::sha2::Sha256>::new_precomputed(
                        context_or_hash,
                    );
                    signer.sign_digest(digest)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp256r1(signer) => match signature_type {
                SignatureType::Secp256r1_PrehashedSha256 => {
                    if context_or_hash.len()
                        != <sha2::Sha256 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha256>::new_precomputed(context_or_hash);
                    signer.sign_digest(digest)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Secp384r1(signer) => match signature_type {
                SignatureType::Secp384r1_PrehashedSha384 => {
                    if context_or_hash.len()
                        != <sha2::Sha384 as sha2::digest::OutputSizeUser>::OutputSize::USIZE
                    {
                        return Err(Error::InvalidArgument);
                    }
                    let digest =
                        digests::DummyDigest::<sha2::Sha384>::new_precomputed(context_or_hash);
                    signer.sign_digest(digest)
                }
                _ => Err(Error::InvalidArgument),
            },
            Self::Sr25519(signer) => match signature_type {
                SignatureType::Sr25519_Pure => signer.sign(context_or_hash, message),
                _ => Err(Error::InvalidArgument),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_signature_conversion() {
        let raw = vec![0x00, 0x01, 0x02, 0x03];
        let sig = Signature::from(raw.clone());
        let v: Vec<u8> = sig.clone().into();
        assert_eq!(v, raw);

        let vref: &[u8] = v.as_ref();
        assert_eq!(vref, sig.as_ref());
    }

    #[test]
    fn test_memory_signer() {
        let ctx = b"oasis-core/test: context";
        let corrupt_ctx = b"oasis-core/test: wrong context";
        let message = b"this is a message";
        let corrupt_message = b"this isn't a message";

        for sig_type in [
            SignatureType::Ed25519_Oasis,
            SignatureType::Ed25519_Pure,
            SignatureType::Secp256k1_Oasis,
            SignatureType::Sr25519_Pure,
        ] {
            let signer = MemorySigner::new_test(sig_type, "memory signer test");
            let pk = signer.public_key();

            let signature = signer
                .sign_by_type(sig_type, ctx, message)
                .expect("signing should succeed");

            pk.verify_by_type(sig_type, ctx, message, &signature)
                .expect("signature should verify");
            pk.verify_by_type(sig_type, ctx, corrupt_message, &signature)
                .expect_err("signature should fail verification");
            if matches!(sig_type, SignatureType::Ed25519_Oasis)
                || matches!(sig_type, SignatureType::Secp256k1_Oasis)
            {
                pk.verify_by_type(sig_type, corrupt_ctx, message, &signature)
                    .expect_err("signature should fail verification");
                pk.verify_by_type(sig_type, corrupt_ctx, corrupt_message, &signature)
                    .expect_err("signature should fail verification");
            }
        }
    }

    #[test]
    fn test_memory_signer_prehashed() {
        let message = b"this is a message";
        let corrupt_message = b"this isn't a message";

        let sig_types: &[(SignatureType, Box<dyn Fn(&[u8]) -> Vec<u8>>)] = &[
            (
                SignatureType::Ed25519_PrehashedSha512,
                Box::new(|message| sha2::Sha512::digest(message).to_vec()),
            ),
            (
                SignatureType::Secp256k1_PrehashedKeccak256,
                Box::new(|message| sha3::Keccak256::digest(message).to_vec()),
            ),
            (
                SignatureType::Secp256k1_PrehashedSha256,
                Box::new(|message| sha2::Sha256::digest(message).to_vec()),
            ),
            (
                SignatureType::Secp256r1_PrehashedSha256,
                Box::new(|message| sha2::Sha256::digest(message).to_vec()),
            ),
            (
                SignatureType::Secp384r1_PrehashedSha384,
                Box::new(|message| sha2::Sha384::digest(message).to_vec()),
            ),
        ];

        for (sig_type, hasher) in sig_types {
            let hash = hasher(message);
            let corrupt_hash = hasher(corrupt_message);

            let signer = MemorySigner::new_test(*sig_type, "memory signer test");
            let pk = signer.public_key();

            let signature = signer
                .sign_by_type(*sig_type, &hash, b"")
                .expect("signing should succeed");
            pk.verify_by_type(*sig_type, &hash, b"", &signature)
                .expect("signature should verify");
            pk.verify_by_type(*sig_type, &corrupt_hash, b"", &signature)
                .expect_err("corrupt hash shouldn't verify");
        }
    }
}
