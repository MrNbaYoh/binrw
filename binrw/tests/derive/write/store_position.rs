use binrw::io::Cursor;
use binrw::{binwrite, BinWrite};

#[test]
fn store_position_writing() {
    #[binwrite]
    struct Test {
        x: u32,
        #[bw(store_position = y_pos)]
        y: u8,
        #[bw(calc = y_pos)]
        z: u64,
    }

    let mut x = Vec::new();
    {
        let mut x = Cursor::new(&mut x);
        Test {
            x: 0xffff_ffff,
            y: 0,
        }
        .write_le(&mut x)
        .unwrap();
    }
    assert_eq!(x, b"\xff\xff\xff\xff\0\x04\0\0\0\0\0\0\0");
}

#[test]
fn store_position_if_cond_writing() {
    #[binwrite]
    struct Test {
        x: u32,
        #[bw(if(*x == 0), store_position = y_pos)]
        y: u8,
        #[bw(calc = y_pos)]
        z: Option<u64>,
    }

    let mut x = Vec::new();
    {
        let mut x = Cursor::new(&mut x);
        Test {
            x: 0xffff_ffff,
            y: 0,
        }
        .write_le(&mut x)
        .unwrap();
    }
    assert_eq!(x, b"\xff\xff\xff\xff");

    let mut x = Vec::new();
    {
        let mut x = Cursor::new(&mut x);
        Test { x: 0, y: 0 }.write_le(&mut x).unwrap();
    }
    assert_eq!(x, b"\0\0\0\0\0\x04\0\0\0\0\0\0\0");
}
