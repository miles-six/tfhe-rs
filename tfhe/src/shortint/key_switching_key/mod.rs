//! This module defines KeySwitchingKey
//!
//! - [KeySwitchingKey] allows switching the keys of a ciphertext, from a cleitn key to another.

use crate::shortint::engine::ShortintEngine;
use crate::shortint::parameters::ShortintKeySwitchingParameters;
use crate::shortint::{Ciphertext, ClientKey, ServerKey};

use crate::core_crypto::prelude::{keyswitch_lwe_ciphertext, LweKeyswitchKeyOwned};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod test;

/// A structure containing the casting public key.
///
/// The casting key is generated by the client and is meant to be published: the client
/// sends it to the server so it can cast from one set of parameters to another.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeySwitchingKey {
    pub(crate) key_switching_key: LweKeyswitchKeyOwned<u64>,
    pub(crate) dest_server_key: ServerKey,
    pub(crate) src_server_key: ServerKey,
    pub cast_rshift: i8,
}

impl KeySwitchingKey {
    /// Generate a casting key. This can cast to several kinds of keys (shortint, integer, hlapi),
    /// depending on input.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tfhe::shortint::parameters::{
    ///     PARAM_MESSAGE_1_CARRY_1_KS_PBS, PARAM_MESSAGE_2_CARRY_2_KS_PBS,
    /// };
    /// use tfhe::shortint::prelude::*;
    /// use tfhe::shortint::{gen_keys, KeySwitchingKey};
    ///
    /// // Generate the client keys and server keys:
    /// let (ck1, sk1) = gen_keys(PARAM_MESSAGE_1_CARRY_1_KS_PBS);
    /// let (ck2, sk2) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);
    ///
    /// // Generate the server key:
    /// let ksk = KeySwitchingKey::new(
    ///     (&ck1, &sk1),
    ///     (&ck2, &sk2),
    ///     PARAM_KEYSWITCH_1_1_KS_PBS_TO_2_2_KS_PBS,
    /// );
    /// ```
    pub fn new(
        key_pair_1: (&ClientKey, &ServerKey),
        key_pair_2: (&ClientKey, &ServerKey),
        params: ShortintKeySwitchingParameters,
    ) -> Self {
        // Creation of the key switching key
        let key_switching_key = ShortintEngine::with_thread_local_mut(|engine| {
            engine.new_key_switching_key(key_pair_1.0, key_pair_2.0, params)
        });

        let full_message_modulus_1 =
            key_pair_1.0.parameters.carry_modulus().0 * key_pair_1.0.parameters.message_modulus().0;
        let full_message_modulus_2 =
            key_pair_2.0.parameters.carry_modulus().0 * key_pair_2.0.parameters.message_modulus().0;
        assert!(
            full_message_modulus_1.is_power_of_two() && full_message_modulus_2.is_power_of_two(),
            "Cannot create casting key if the full messages moduli are not a power of 2"
        );

        let nb_bits_1: i8 = full_message_modulus_1.ilog2().try_into().unwrap();
        let nb_bits_2: i8 = full_message_modulus_2.ilog2().try_into().unwrap();

        // Pack the keys in the casting key set:
        Self {
            key_switching_key,
            dest_server_key: key_pair_2.1.clone(),
            src_server_key: key_pair_1.1.clone(),
            cast_rshift: nb_bits_2 - nb_bits_1,
        }
    }

    /// Deconstruct a [`KeySwitchingKey`] into its constituents.
    pub fn into_raw_parts(self) -> (LweKeyswitchKeyOwned<u64>, ServerKey, ServerKey, i8) {
        let Self {
            key_switching_key,
            dest_server_key,
            src_server_key,
            cast_rshift,
        } = self;

        (
            key_switching_key,
            dest_server_key,
            src_server_key,
            cast_rshift,
        )
    }

    /// Construct a [`KeySwitchingKey`] from its constituents.
    ///
    /// # Panics
    ///
    /// Panics if the provided raw parts are not compatible with each other, i.e.:
    ///
    /// if the provided source [`ServerKey`] ciphertext
    /// [`LweDimension`](`crate::core_crypto::commons::parameters::LweDimension`) does not match the
    /// input [`LweDimension`](`crate::core_crypto::commons::parameters::LweDimension`) of the
    /// provided [`LweKeyswitchKeyOwned`] or if the provided destination [`ServerKey`]
    /// ciphertext [`LweDimension`](`crate::core_crypto::commons::parameters::LweDimension`)
    /// does not match the output
    /// [`LweDimension`](`crate::core_crypto::commons::parameters::LweDimension`) of the
    /// provided [`LweKeyswitchKeyOwned`].
    pub fn from_raw_parts(
        key_switching_key: LweKeyswitchKeyOwned<u64>,
        dest_server_key: ServerKey,
        src_server_key: ServerKey,
        cast_rshift: i8,
    ) -> Self {
        let src_lwe_dimension = src_server_key.ciphertext_lwe_dimension();
        let dst_lwe_dimension = dest_server_key.ciphertext_lwe_dimension();

        assert_eq!(
            src_lwe_dimension,
            key_switching_key.input_key_lwe_dimension(),
            "Mismatch between the source ServerKey ciphertext LweDimension ({:?}) \
            and the LweKeyswitchKey input LweDimension ({:?})",
            src_lwe_dimension,
            key_switching_key.input_key_lwe_dimension(),
        );
        assert_eq!(
            dst_lwe_dimension,
            key_switching_key.output_key_lwe_dimension(),
            "Mismatch between the destination ServerKey ciphertext LweDimension ({:?}) \
            and the LweKeyswitchKey output LweDimension ({:?})",
            dst_lwe_dimension,
            key_switching_key.output_key_lwe_dimension(),
        );
        assert_eq!(
            src_server_key.ciphertext_modulus, dest_server_key.ciphertext_modulus,
            "Mismatch between the source ServerKey CiphertextModulus ({:?}) \
            and the destination ServerKey CiphertextModulus ({:?})",
            src_server_key.ciphertext_modulus, dest_server_key.ciphertext_modulus,
        );
        assert_eq!(
            key_switching_key.ciphertext_modulus(),
            dest_server_key.ciphertext_modulus,
            "Mismatch between the LweKeyswitchKey CiphertextModulus ({:?}) \
            and the destination ServerKey CiphertextModulus ({:?})",
            key_switching_key.ciphertext_modulus(),
            dest_server_key.ciphertext_modulus,
        );

        Self {
            key_switching_key,
            dest_server_key,
            src_server_key,
            cast_rshift,
        }
    }

    /// Cast a ciphertext from the source parameter set to the dest parameter set,
    /// using provided &mut.
    ///
    /// # Example (the following code won't actually run because this function is private)
    ///
    /// ```rust
    /// use tfhe::shortint::parameters::{
    ///     PARAM_MESSAGE_1_CARRY_1_KS_PBS, PARAM_MESSAGE_2_CARRY_2_KS_PBS,
    /// };
    /// use tfhe::shortint::prelude::*;
    /// use tfhe::shortint::{gen_keys, KeySwitchingKey};
    ///
    /// // Generate the client keys and server keys:
    /// let (ck1, sk1) = gen_keys(PARAM_MESSAGE_1_CARRY_1_KS_PBS);
    /// let (ck2, sk2) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);
    ///
    /// // Generate the server key:
    /// let ksk = KeySwitchingKey::new(
    ///     (&ck1, &sk1),
    ///     (&ck2, &sk2),
    ///     PARAM_KEYSWITCH_1_1_KS_PBS_TO_2_2_KS_PBS,
    /// );
    ///
    /// let cleartext = 1;
    /// let cipher = ck1.encrypt(cleartext);
    /// let mut cipher_2 = sk2.create_trivial(0);
    /// ksk.cast_into(&cipher, &mut cipher_2);
    ///
    /// assert_eq!(ck2.decrypt(&cipher_2), cleartext);
    /// ```
    pub fn cast_into(&self, ct: &Ciphertext, ct_dest: &mut Ciphertext) {
        match self.cast_rshift {
            // Same bit size: only key switch
            0 => keyswitch_lwe_ciphertext(&self.key_switching_key, &ct.ct, &mut ct_dest.ct),

            // Cast to bigger bit length: keyswitch, then right shift
            i if i > 0 => {
                keyswitch_lwe_ciphertext(&self.key_switching_key, &ct.ct, &mut ct_dest.ct);

                let acc = self.dest_server_key.generate_lookup_table(|n| n >> i);
                self.dest_server_key
                    .apply_lookup_table_assign(ct_dest, &acc);
            }

            // Cast to smaller bit length: left shift, then keyswitch
            i if i < 0 => {
                // We want to avoid the padding bit to be dirty, hence the modulus
                let acc = self.src_server_key.generate_lookup_table(|n| {
                    (n << -i) % (ct.carry_modulus.0 * ct.message_modulus.0) as u64
                });
                let shifted_cipher = self.src_server_key.apply_lookup_table(ct, &acc);

                keyswitch_lwe_ciphertext(
                    &self.key_switching_key,
                    &shifted_cipher.ct,
                    &mut ct_dest.ct,
                );
            }

            _ => unreachable!(),
        };
    }

    /// Cast a ciphertext from the source parameter set to the dest parameter set,
    /// returning a new ciphertext.
    ///
    /// # Example (the following code won't actually run because this function is private)
    ///
    /// ```rust
    /// use tfhe::shortint::parameters::{
    ///     PARAM_MESSAGE_1_CARRY_1_KS_PBS, PARAM_MESSAGE_2_CARRY_2_KS_PBS,
    /// };
    /// use tfhe::shortint::prelude::*;
    /// use tfhe::shortint::{gen_keys, KeySwitchingKey};
    ///
    /// // Generate the client keys and server keys:
    /// let (ck1, sk1) = gen_keys(PARAM_MESSAGE_1_CARRY_1_KS_PBS);
    /// let (ck2, sk2) = gen_keys(PARAM_MESSAGE_2_CARRY_2_KS_PBS);
    ///
    /// // Generate the server key:
    /// let ksk = KeySwitchingKey::new(
    ///     (&ck1, &sk1),
    ///     (&ck2, &sk2),
    ///     PARAM_KEYSWITCH_1_1_KS_PBS_TO_2_2_KS_PBS,
    /// );
    ///
    /// let cleartext = 1;
    ///
    /// let cipher = ck1.encrypt(cleartext);
    /// let cipher_2 = ksk.cast(&cipher);
    ///
    /// assert_eq!(ck2.decrypt(&cipher_2), cleartext);
    /// ```
    pub fn cast(&self, ct: &Ciphertext) -> Ciphertext {
        let mut ret = self.dest_server_key.create_trivial(0);
        self.cast_into(ct, &mut ret);
        ret
    }
}
