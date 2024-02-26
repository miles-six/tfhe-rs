use super::*;
use crate::core_crypto::commons::numeric::{Numeric, UnsignedInteger};

/// The distribution $TUniform(1, -2^b, 2^b)$ is defined as follows, any value in the interval
/// $\left[-2^b, 2^b\right]$ is selected with probability $\frac{1}{2^{b+1}}$, with the two end
/// points $-2^b$ and $2^b$ being selected with probability $\frac{1}{2^{b+2}}$.
#[derive(Copy, Clone)]
pub struct TUniform<T: UnsignedInteger> {
    bound_log2: u32,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: UnsignedInteger> TUniform<T> {
    /// Construct a TUniform distribution see [`TUniform`] for behavior of randomly generated
    /// values.
    ///
    /// # Panics
    ///
    /// Panics if `bound_log2` is greater than the Scalar type number of bits minus two.
    ///
    /// The reason for this is that with a higher `bound_log2` it is impossible to distinguish
    /// between $-2^{bound\_log2}$ and $2^{bound\_log2}$ because of the 2's complement
    /// representation of integers.
    pub const fn new(bound_log2: u32) -> Self {
        assert!(
            (bound_log2 + 2) as usize <= T::BITS,
            "bound_log2 + 2 is greater than the current type's bit width"
        );

        Self {
            bound_log2,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn bound_log2(&self) -> u32 {
        self.bound_log2
    }

    pub const fn distinct_value_count(&self) -> usize {
        (1 << (self.bound_log2 + 1)) + 1
    }

    pub fn min_value_inclusive(&self) -> T::Signed {
        -(T::Signed::ONE << self.bound_log2 as usize)
    }

    pub fn max_value_inclusive(&self) -> T::Signed {
        T::Signed::ONE << self.bound_log2 as usize
    }
}

macro_rules! implement_t_uniform_uint {
    ($T:ty) => {
        impl RandomGenerable<TUniform<$T>> for $T {
            type CustomModulus = $T;
            #[allow(unused)]
            fn generate_one<G: ByteRandomGenerator>(
                generator: &mut RandomGenerator<G>,
                TUniform { bound_log2, .. }: TUniform<$T>,
            ) -> Self {
                let mut buf = [0; std::mem::size_of::<$T>()];
                let required_bits = bound_log2 + 2;
                let required_bytes = required_bits.div_ceil(u8::BITS) as usize;
                let mod_mask = <$T>::MAX >> (<$T>::BITS - required_bits);

                // For small moduli compared to the native type allows to avoid wasting too much
                // random bytes generated by the CSPRNG.
                buf.iter_mut()
                    .take(required_bytes)
                    .for_each(|a| *a = generator.generate_next());
                // We use from_le_bytes as most platforms are low endian, this avoids endianness
                // issues
                let native_int_random = <$T>::from_le_bytes(buf);
                let mut candidate_for_random = native_int_random & mod_mask;
                let bit_b_p_1 = candidate_for_random & 1;
                candidate_for_random >>= 1;
                candidate_for_random = candidate_for_random.wrapping_add(bit_b_p_1);
                candidate_for_random.wrapping_sub(1 << bound_log2)
            }
        }
    };
}

implement_t_uniform_uint!(u8);
implement_t_uniform_uint!(u16);
implement_t_uniform_uint!(u32);
implement_t_uniform_uint!(u64);
implement_t_uniform_uint!(u128);
