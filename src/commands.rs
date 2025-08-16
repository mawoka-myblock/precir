use alloc::vec::Vec;

pub fn get_image_parameter_frame(
    plid: [u8; 4],
    width: u16,
    height: u16,
    x_pos: u16,
    y_pos: u16,
    img_len: u16,
) -> Vec<u8> {
    let mut frame: Vec<u8> = Vec::new();
    let mut frame: Vec<u8> = Vec::new();
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x40]); // Header
    frame.push(0x85); // VER
    frame.extend_from_slice(&[plid[3], plid[2], plid[1], plid[0]]);
    frame.push(0x34); // CMD: image
    frame.extend_from_slice(&0x00000005u32.to_be_bytes()); // placeholder length, can be 5
    frame.extend_from_slice(&img_len.to_be_bytes()); // LENGTH of image data
    frame.push(0x00); // Unused
    frame.push(0x00); // TYPE: 0 = raw
    frame.push(0x01); // PAGE (optional, usually 1)
    frame.extend_from_slice(&width.to_be_bytes());
    frame.extend_from_slice(&height.to_be_bytes());
    frame.extend_from_slice(&x_pos.to_be_bytes());
    frame.extend_from_slice(&y_pos.to_be_bytes());
    frame.push(0x00); // KEYcode
    frame.push(0x00); // KEYcode
    frame.push(0x88); // unknown/marker?
    frame.extend_from_slice(&[0u8; 6]); // reserved
    let crc = crc16(&frame[4..]);
    frame.push((crc & 0xFF) as u8);
    frame.push((crc >> 8) as u8);
    frame
}

/// Split image data into 20-byte data frames
pub fn build_data_frames(plid: [u8; 4], img_data: &[u8]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut index = 0u8;

    for chunk in img_data.chunks(20) {
        let mut frame: Vec<u8> = Vec::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x40]);
        frame.push(0x85);
        frame.extend_from_slice(&[plid[3], plid[2], plid[1], plid[0]]);
        frame.push(0x34); // CMD
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]); // length 32? often fixed
        frame.extend_from_slice(&(index as u16).to_be_bytes()); // INDEX
        frame.extend_from_slice(chunk); // DATA
        let crc = crc16(&frame[4..]);
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);
        frames.push(frame);
        index += 1;
    }

    frames
}

/// Final update frame
pub fn get_final_frame(plid: [u8; 4]) -> Vec<u8> {
    let mut frame: Vec<u8> = Vec::new();
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x40]);
    frame.push(0x85);
    frame.extend_from_slice(&[plid[3], plid[2], plid[1], plid[0]]);
    frame.push(0x34); // CMD
    frame.extend_from_slice(&0x00000001u32.to_be_bytes()); // length 1?
    frame.extend_from_slice(&[0u8; 22]); // PAYLOAD
    let crc = crc16(&frame[4..]);
    frame.push((crc & 0xFF) as u8);
    frame.push((crc >> 8) as u8);
    frame
}
pub fn get_wakeup_command(plid: [u8; 4]) -> Vec<u8> {
    let mut frame: Vec<u8> = Vec::new();
    // PP16 header required
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x40]);

    // Frame body
    frame.push(0x85); // VER for graphic ESLs

    // PLID (LSB first) – replace with your target PLID
    frame.extend_from_slice(&[plid[3], plid[2], plid[1], plid[0]]); // example PLID

    frame.push(0x17); // CMD: command code with ack flag (bit 7 = 0 here)
    frame.push(0x01); // PARAMETER: unknown purpose, often 0x01

    // KEY (16-bit, LSB first) – often 0x0000
    frame.extend_from_slice(&[0x00, 0x00]);
    frame.push(0x00);

    for _ in 0..22 {
        frame.push(0x01);
    }

    // CRC16 over VER..payload (skip PP16)
    let crc = crc16(&frame[4..]);
    frame.push((crc & 0xFF) as u8);
    frame.push((crc >> 8) as u8);

    frame
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x8408;
    let poly: u16 = 0x8408;

    for &b in data {
        crc ^= b as u16; // XOR the byte into the low 8 bits
        for _ in 0..8 {
            if (crc & 0x0001) != 0 {
                crc = (crc >> 1) ^ poly;
            } else {
                crc >>= 1;
            }
        }
    }

    crc // <-- DO NOT invert
}
