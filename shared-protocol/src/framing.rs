use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{Result, Error, ErrorKind};

/// Write a length-prefixed frame to any async writer.
pub async fn write_frame<W: AsyncWriteExt + Unpin>(stream: &mut W, data: &[u8]) -> Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(data).await?;
    Ok(())
}

/// Reads a frame into the provided buffer. Returns the number of bytes read.
pub async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R, buf: &mut Vec<u8>) -> Result<Option<usize>> {
    let mut len_bytes = [0u8; 4];
    match reader.read_exact(&mut len_bytes).await {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    if len > 1_048_576 { // 1mb
        return Err(Error::new(ErrorKind::InvalidData, "frame too large"));
    }
    
    buf.resize(len, 0);
    reader.read_exact(&mut buf[..len]).await?;
    reader.read_exact(buf).await?;
    Ok(Some(len))
}