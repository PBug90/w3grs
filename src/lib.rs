use bytebuffer::*;
mod w3grs {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::env;
    use std::fs;
    use std::io::Read;
    use std::str;
    use std::time::Instant;
    pub struct ParserResult {
        pub header: Header,
        pub subheader: Subheader,
        pub metadata: MapMetadata,
    }
    pub struct Header {
        compressed_size: i32,
        pub header_version: i32,
        decompressed_size: i32,
        compressed_data_blocks: i32,
    }

    pub struct Subheader {
        pub game_identifier: String,
        pub version: i32,
        pub build_no: i16,
        pub replay_length_ms: i32,
    }

    struct DataBlock {
        size: u16,
        decompressed_size: u16,
        content: Vec<u8>,
    }

    struct PlayerRecord {
        id: i8,
        name: String,
    }

    struct ReforgedPlayerRecord {
        id: u8,
        name: String,
        clan: String,
    }

    pub struct MapMetadata {
        pub map: String,
        pub teams_together: bool,
        pub random_hero: bool,
        pub random_races: bool,
        pub map_explored: bool,
        pub hide_terrain: bool,
        pub always_visible: bool,
    }

    fn decode_game_meta_data(data: Vec<u8>) -> Vec<u8> {
        let mut decoded = Vec::new();
        let mut mask = 0;

        for a in 0..data.len() {
            if a % 8 == 0 {
                mask = data[a];
            } else {
                if mask & (0x1 << a % 8) == 0 {
                    decoded.push(data[a] - 1);
                } else {
                    decoded.push(data[a]);
                }
            }
        }
        return decoded;
    }

    fn read_zero_terminated(buf: &mut ByteBuffer) -> String {
        let mut buffer = Vec::new();
        let mut b = buf.read_u8().unwrap();
        while b != 0 {
            buffer.push(b);
            b = buf.read_u8().unwrap();
        }
        if buffer.len() > 0 {
            let mut the_string = String::new();
            the_string.push_str(&String::from_utf8_lossy(&buffer));
            the_string
        } else {
            String::new()
        }
    }

    fn read_data_zeroterminated(buf: &mut ByteBuffer) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut b = buf.read_u8().unwrap();
        while b != 0 {
            buffer.push(b);
            b = buf.read_u8().unwrap();
        }
        return buffer;
    }

    fn read_header(buf: &mut ByteBuffer) -> Header {
        let compressed_size = buf.read_i32().unwrap();
        let header_version = buf.read_i32().unwrap();
        let decompressed_size = buf.read_i32().unwrap();
        let compressed_data_blocks = buf.read_i32().unwrap();

        let header = Header {
            compressed_size: compressed_size,
            header_version: header_version,
            decompressed_size: decompressed_size,
            compressed_data_blocks: compressed_data_blocks,
        };
        header
    }

    fn read_subheader(buf: &mut ByteBuffer) -> Subheader {
        let game_identifier = buf.read_bytes(4).unwrap();
        let mut the_string = String::new();
        the_string.push_str(str::from_utf8(&game_identifier).expect("not UTF-8"));

        let version = buf.read_i32().unwrap();
        let build_no = buf.read_i16().unwrap();

        buf.set_rpos(buf.get_rpos() + 2);
        let replay_length_ms = buf.read_i32().unwrap();
        buf.set_rpos(buf.get_rpos() + 4);

        let subheader = Subheader {
            game_identifier: the_string,
            version: version,
            build_no: build_no,
            replay_length_ms: replay_length_ms,
        };
        subheader
    }

    fn read_playerrecord(buf: &mut ByteBuffer) -> PlayerRecord {
        let id = buf.read_i8().unwrap();
        let name = read_zero_terminated(buf);
        let add_data = buf.read_u8().unwrap();
        if add_data == 1 {
            buf.read_u8().unwrap();
        } else if add_data == 2 {
            buf.read_u16().unwrap();
        } else if add_data == 8 {
            buf.read_u64().unwrap();
        }
        let record = PlayerRecord { name: name, id: id };
        record
    }

    fn read_string_of_length(buf: &mut ByteBuffer, length: usize) -> String {
        let mut the_string = String::new();
        the_string.push_str(str::from_utf8(&buf.read_bytes(length).unwrap()).expect("not UTF-8"));
        return the_string;
    }

    fn read_mapmetadata(metadata: Vec<u8>) -> MapMetadata {
        let mut a = ByteBuffer::from_bytes(&metadata);
        a.set_endian(Endian::LittleEndian);
        let speed = a.read_u8().unwrap();
        let teams_together = a.read_bit().unwrap();
        let observer_mode = a.read_bits(2).unwrap();
        let default = a.read_bit().unwrap();
        let always_visible = a.read_bit().unwrap();
        let map_explored = a.read_bit().unwrap();
        let hide_terrain = a.read_bit().unwrap();
        a.read_bit().unwrap();

        let fixed_teams = a.read_bits(2).unwrap();
        a.read_bits(6).unwrap();

        a.read_bit().unwrap();
        let referees = a.read_bit().unwrap();
        a.read_bits(3).unwrap();
        let random_races = a.read_bit().unwrap();
        let random_hero = a.read_bit().unwrap();
        let full_shared_unit_control = a.read_bit().unwrap();
        a.read_bytes(5).unwrap();
        let checksum = a.read_bytes(4).unwrap();

        let map = read_zero_terminated(&mut a);
        let creator = read_zero_terminated(&mut a);
        return MapMetadata {
            map: map,
            random_hero: random_hero,
            random_races: random_races,
            map_explored: map_explored,
            hide_terrain: hide_terrain,
            always_visible: always_visible,
            teams_together: teams_together,
        };
    }

    fn read_blocks(buf: &mut ByteBuffer) -> Vec<DataBlock> {
        let mut data_blocks: Vec<DataBlock> = Vec::new();
        while buf.get_rpos() < buf.len() {
            let size = buf.read_u16().unwrap();
            buf.read_i16().unwrap();
            let decompressed_size = buf.read_u16().unwrap();
            buf.read_i32().unwrap();
            buf.read_i16().unwrap();
            let content = buf.read_bytes(size.into()).unwrap();
            let block = DataBlock {
                decompressed_size: decompressed_size,
                size: size,
                content: content,
            };
            data_blocks.push(block);
        }
        data_blocks
    }

    fn parse_reforged_metadata(buf: &mut ByteBuffer) -> Vec<ReforgedPlayerRecord> {
        let mut result: Vec<ReforgedPlayerRecord> = Vec::new();
        buf.set_rpos(buf.get_rpos() + 12);
        let mut attempts = 0;
        while buf.read_u8().unwrap() != 25 && attempts < 24 {
            let record_length = buf.read_u8().unwrap();
            let record_end = buf.get_rpos() + record_length as usize;
            buf.read_u8().unwrap();
            let id = buf.read_u8().unwrap();
            buf.read_u8().unwrap();
            let name_length = buf.read_u8().unwrap();
            let name = read_string_of_length(buf, name_length as usize);
            buf.read_u8().unwrap();
            let clan_length = buf.read_u8().unwrap();
            let clan_name = read_string_of_length(buf, clan_length as usize);
            buf.set_rpos(record_end);
            attempts += 1;
            result.push(ReforgedPlayerRecord {
                id: id,
                clan: clan_name,
                name: name,
            });
        }
        return result;
    }

    fn parse_slot_record(buf: &mut ByteBuffer) {
        let player_id = buf.read_u8().unwrap();
        buf.read_u8().unwrap();
        let status = buf.read_u8().unwrap();
        let computer_flag = buf.read_u8().unwrap();
        let team_id = buf.read_u8().unwrap();
        let color = buf.read_u8().unwrap();
        let race_flag = buf.read_u8().unwrap();
        let ai_strength = buf.read_u8().unwrap();
        let handicap_flag = buf.read_u8().unwrap();
    }
    #[allow(dead_code)]
    pub fn parse(filename: String) -> ParserResult {
        // --snip--
        let start = Instant::now();
        let args: Vec<String> = env::args().collect();

        let mut file = match fs::File::open(filename) {
            Ok(f) => f,
            Err(e) => {
                use std::io::ErrorKind::*;
                println!("Got error: {}", e);
                match e.kind() {
                    NotFound => {
                        println!("File not found");
                    }
                    k => {
                        println!("Error: {:?}", k);
                    }
                }
                panic!("asd");
            }
        };
        let mut buffer = Vec::new();
        match file.read_to_end(&mut buffer) {
            Ok(f) => f,
            Err(e) => {
                use std::io::ErrorKind::*;
                println!("Got error: {}", e);
                match e.kind() {
                    NotFound => {
                        println!("File not found");
                    }
                    k => {
                        println!("Error: {:?}", k);
                    }
                }
                panic!("asd")
            }
        };

        let (nothing, rest) = buffer.split_at(0);
        let mut b = ByteBuffer::from_bytes(rest);
        b.set_endian(Endian::LittleEndian);
        let s = read_zero_terminated(&mut b);
        b.read_i32().unwrap();
        let header = read_header(&mut b);
        let subheader = read_subheader(&mut b);
        let mut data_blocks = read_blocks(&mut b);
        let mut out = Vec::new();
        for (i, elem) in data_blocks.iter_mut().enumerate() {
            let mut d = ZlibDecoder::new(elem.content.as_slice());
            let mut out2 = Vec::new();
            d.read_to_end(&mut out2).unwrap();
            out.append(&mut out2);
        }

        let mut meta_parser = ByteBuffer::from_bytes(out.as_slice());
        meta_parser.set_endian(Endian::LittleEndian);
        meta_parser.read_u32().unwrap();
        meta_parser.read_u8().unwrap();
        let record = read_playerrecord(&mut meta_parser);
        let game_name = read_zero_terminated(&mut meta_parser);
        let private = read_zero_terminated(&mut meta_parser);
        let encoded_mapmeta = read_data_zeroterminated(&mut meta_parser);
        let decoded_mapmeta = decode_game_meta_data(encoded_mapmeta);
        let metadata = read_mapmetadata(decoded_mapmeta);
        meta_parser.read_u32().unwrap();
        meta_parser.read_u32().unwrap();
        meta_parser.read_u32().unwrap();
        while meta_parser.read_u8().unwrap() == 22 {
            let record = read_playerrecord(&mut meta_parser);
            println!("{} {}", record.name, record.id);
            meta_parser.read_u32().unwrap();
        }
        meta_parser.set_rpos(meta_parser.get_rpos() - 1);
        if meta_parser.read_u8().unwrap() != 25 {
            meta_parser.set_rpos(meta_parser.get_rpos() - 1);
            parse_reforged_metadata(&mut meta_parser);
        }
        meta_parser.set_rpos(meta_parser.get_rpos() + 2);
        let slot_record_count = meta_parser.read_u8().unwrap();
        for x in 0..slot_record_count {
            parse_slot_record(&mut meta_parser);
        }
        let random_seed = meta_parser.read_u32().unwrap();
        meta_parser.read_u8().unwrap();
        let start_spot_count = meta_parser.read_u8().unwrap();
        println!("{}", start_spot_count);
        let duration = start.elapsed();
        println!("Took {}", duration.as_millis());
        let result = ParserResult {
            header: header,
            subheader: subheader,
            metadata: metadata,
        };
        return result;
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_reforged1() {
        // This assert would fire and test will fail.
        // Please note, that private functions can be tested too!
        let p = w3grs::parse(String::from("replays/reforged1.w3g"));
        assert_eq!(p.header.header_version, 1);
        assert_eq!(p.subheader.game_identifier, "PX3W");
        assert_eq!(
            p.metadata.map,
            "Maps/Download/d57df8794b66784681a0ba4a3295b4aef142fde4/(2)TerenasStand_LV.w3x"
        );
        assert_eq!(p.metadata.map_explored, false);
        assert_eq!(p.metadata.random_hero, false);
        assert_eq!(p.metadata.random_races, false);
    }
}
