pub fn checksum(bytes: &[u8]) -> u16 {
    let length = bytes.len();
    let mut checksum = 0u32;
    // パケットの各 2 バイトを 16 ビットの整数として足し合わせる
    for i in (0..length).step_by(2) {
        checksum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
    }
    // 合計が 16 ビットを超えている場合、上位 16 ビットと下位 16 ビットを足し合わせる
    // 0xFFFF は 16 ビットの最大値、checksum >> 16 は上位 16 ビット、checksum & 0xFFFF は下位 16 ビットを取得する
    while checksum > 0xFFFF {
        checksum = (checksum & 0xFFFF) + (checksum >> 16);
    }
    // 1 の補数を取る
    0xFFFF - checksum as u16
}
