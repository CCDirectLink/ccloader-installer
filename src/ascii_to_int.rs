pub trait IntFromAsciiHelper: Sized {
  fn from_u8(value: u8) -> Self;
  fn checked_mul(&self, other: u8) -> Option<Self>;
  fn checked_add(&self, other: u8) -> Option<Self>;
}

macro_rules! impl_IntFromAsciiHelper {
  ($($t:ty)*) => ($(impl IntFromAsciiHelper for $t {
    #[inline]
    fn from_u8(value: u8) -> Self {
      value as Self
    }

    #[inline]
    fn checked_mul(&self, other: u8) -> Option<Self> {
      Self::checked_mul(*self, other as Self)
    }

    #[inline]
    fn checked_add(&self, other: u8) -> Option<Self> {
      Self::checked_add(*self, other as Self)
    }
  })*)
}
impl_IntFromAsciiHelper! { u64 usize }

pub fn ascii_to_int<T: IntFromAsciiHelper>(bytes: &[u8]) -> Option<T> {
  if bytes.is_empty() {
    return None;
  }
  let mut result = T::from_u8(0);
  for byte in bytes {
    if !(b'0' <= *byte && *byte <= b'9') {
      return None;
    }
    let digit = byte - b'0';
    result = result.checked_mul(10)?.checked_add(digit)?;
  }
  Some(result)
}
