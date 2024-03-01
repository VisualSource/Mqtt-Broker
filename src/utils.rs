use crate::headers::fixed_header::MqttFixedHeader;

pub trait FromBytes {
    type Output;
    fn from_bytes<'a, I>(iter: I, header: MqttFixedHeader) -> Result<Self::Output, ()>
    where
        I: Iterator<Item = &'a u8>;
}

pub fn unpack_u16<'a, I>(iter: &mut I) -> Result<u16, ()>
where
    I: Iterator<Item = &'a u8>,
{
    let msb = iter.next().ok_or_else(|| ())?;
    let lsb = iter.next().ok_or_else(|| ())?;
    Ok(((msb.to_owned() as u16) << 8) | lsb.to_owned() as u16)
}

pub fn unpack_string<'a, I>(iter: &mut I) -> Result<String, ()>
where
    I: Iterator<Item = &'a u8>,
{
    let len = usize::from(unpack_u16(iter)?);

    let mut chars = Vec::with_capacity(len);

    let mut idx = 0;
    while idx < len {
        chars.push(iter.next().ok_or_else(|| ())?.to_owned());
        idx += 1;
    }

    let result = String::from_utf8(chars).map_err(|_| ())?;

    Ok(result)
}
