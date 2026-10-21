
#[cfg(test)] mod tests {
#[test] fn byte_and_substring_boundaries() {
 for offset in 0..64 { for len in [0,1,2,3,15,16,17,31,32,33,63,64,65,255,256,257] {
  let storage: Vec<u8>=(0..offset+len).map(|i| ((i*37+i/11)%251) as u8).collect(); let h=&storage[offset..];
  for a in [0u8,1,63,127,250,255] { let b=a.wrapping_add(1); let c=a.wrapping_add(2);
   assert_eq!(memchr::memchr(a,h),h.iter().position(|&v|v==a)); assert_eq!(memchr::memrchr(a,h),h.iter().rposition(|&v|v==a));
   assert_eq!(memchr::memchr2(a,b,h),h.iter().position(|&v|v==a||v==b)); assert_eq!(memchr::memrchr2(a,b,h),h.iter().rposition(|&v|v==a||v==b));
   assert_eq!(memchr::memchr3(a,b,c,h),h.iter().position(|&v|v==a||v==b||v==c)); assert_eq!(memchr::memrchr3(a,b,c,h),h.iter().rposition(|&v|v==a||v==b||v==c));
  }
  for nlen in [0,1,2,3,7,16,32,64,128,258] { let needle = &storage[..nlen.min(storage.len())];
   let expected=if needle.is_empty(){Some(0)}else{h.windows(needle.len()).position(|x|x==needle)};
   let reverse=if needle.is_empty(){Some(h.len())}else{h.windows(needle.len()).rposition(|x|x==needle)};
   assert_eq!(memchr::memmem::find(h,needle),expected); assert_eq!(memchr::memmem::rfind(h,needle),reverse);
   assert_eq!(memchr::memmem::Finder::new(needle).find(h),expected);
  }
 }}
}
#[tokio::test] async fn linux_process_and_unix_ipc() {
 use tokio::io::{AsyncReadExt,AsyncWriteExt};
 let output=tokio::process::Command::new("/bin/sh").args(["-c","printf baseline"]).output().await.unwrap(); assert!(output.status.success()); assert_eq!(output.stdout,b"baseline");
 let mut child=tokio::process::Command::new("/bin/sleep").arg("30").spawn().unwrap(); child.kill().await.unwrap(); assert!(!child.wait().await.unwrap().success());
 let (mut a,mut b)=tokio::net::UnixStream::pair().unwrap(); a.write_all(b"ipc").await.unwrap(); let mut bytes=[0;3]; b.read_exact(&mut bytes).await.unwrap(); assert_eq!(&bytes,b"ipc");
}
}
