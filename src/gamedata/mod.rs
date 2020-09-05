pub mod gamedata {
    use crate::read_zero_terminated;
    use bytebuffer::ByteBuffer;

    pub fn parse(buf: &mut ByteBuffer) {
        while buf.get_rpos() < buf.len() {
            let id = buf.read_u8().unwrap();
            match id {
                0x1a => consume_bytes(4, buf),
                0x1b => consume_bytes(4, buf),
                0x1c => consume_bytes(4, buf),
                0x1f => parse_timeslot(buf),
                0x17 => parse_leave_game(buf),
                0x20 => parse_chat_message(buf),
                0x22 => parse_unknown_22(buf),
                0x23 => consume_bytes(10, buf),
                0x2f => consume_bytes(8, buf),
                _ => (),
            };
        }
    }

    fn parse_unknown_22(buf: &mut ByteBuffer) {
        let length = buf.read_u8().unwrap();
        buf.read_bytes(length as usize).unwrap();
    }

    fn parse_timeslot(buf: &mut ByteBuffer) {
        let byte_count = buf.read_u16().unwrap();
        buf.read_bytes(byte_count as usize).unwrap();
    }

    fn consume_bytes(amount: u32, buf: &mut ByteBuffer) {
        buf.read_bytes(amount as usize).unwrap();
    }

    fn parse_leave_game(buf: &mut ByteBuffer) {
        let reason = buf.read_bytes(4).unwrap();
        let player_id = buf.read_u8().unwrap();
        let result = buf.read_bytes(4).unwrap();
        buf.read_bytes(4).unwrap();
    }

    fn parse_chat_message(buf: &mut ByteBuffer) {
        let byte_count = buf.read_u8().unwrap();
        let flags = buf.read_u8().unwrap();
        if flags == 0x20 {
            buf.read_bytes(4).unwrap();
        }
        let message = read_zero_terminated(buf);
    }
}
