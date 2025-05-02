use crate::messages::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_vsock::VsockStream;

/// Read the next frame from the stream. Each frame is prefixed with its u64 length (little-endian).
pub async fn read_next_frame(stream: &mut VsockStream) -> anyhow::Result<Vec<u8>> {
    let mut len_buf = [0u8; size_of::<u64>()];
    stream.read_exact(&mut len_buf).await?;

    let len = u64::from_le_bytes(len_buf);
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).await?;

    Ok(buf)
}

/// Write the given buffer to the stream, prefixed with its u64 length (little-endian).
/// This is the counterpart to `read_next_frame`.
pub async fn write_frame(stream: &mut VsockStream, buf: &[u8]) -> anyhow::Result<()> {
    let len = buf.len() as u64;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(buf).await?;
    Ok(())
}

pub async fn read_next_message(stream: &mut VsockStream) -> anyhow::Result<Message> {
    let buf = read_next_frame(stream).await?;
    let message = bincode::deserialize(&buf)?;
    Ok(message)
}

pub async fn write_message(stream: &mut VsockStream, message: &Message) -> anyhow::Result<()> {
    let buf = bincode::serialize(message)?;
    write_frame(stream, &buf).await?;
    Ok(())
}
