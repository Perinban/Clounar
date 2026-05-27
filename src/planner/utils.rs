pub fn sanitize(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            i += 1;
        } else {
            s.push(bytes[i] as char);
            i += 1;
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
