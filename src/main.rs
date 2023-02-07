use clap::Parser;
use byteorder::{LittleEndian, ByteOrder};
use std::io::{Read, Write};
use std::fs::File;
use std::path::Path;

fn read_file<P: AsRef<Path>>(file_path: P) -> Vec<u8> {
    let mut file = File::open(file_path).expect("open file failed");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read file failed");
    buf
}

fn write_file<P: AsRef<Path>>(file_path: P, buf: &[u8]) {
    let mut file = File::create(file_path).expect("create file failed");
    file.write_all(buf).expect("write file failed");
}

fn injection_code(e_entry: u64, program_size: usize) -> Vec<u8> {
    // 注入するコード本体
    // 実行するコードのオフセットと元のコードへの復帰場所を追記したものをこの関数から返す
    let mut code = [
        0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, 0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00, 0x48, 0xc7, 
        0xc6, 0xe0, 0x00, 0x40, 0x00, 0x48, 0xc7, 0xc2, 0x09, 0x00, 0x00, 0x00, 0x0f, 0x05, 0xeb, 0x00,
        0x49, 0x6e, 0x6a, 0x65, 0x63, 0x74, 0x65, 0x64, 0x0a, 0x00
    ];

    // 実行対象のコードのオフセットを追記する
    // 17 ~ 20 のバイトが呼び出す文字列の位置を指定している
    // ここでは32バイト目（Injected\n文字列の開始位置）をになるように計算している
    let offset = 0x00400000;  // メモリ配置時のオフセット
    let code_offset = 32;  // Injected\n の文字が配置してある場所までのオフセット
    let code_pointer = program_size as u64 + code_offset + offset;
    let mut code_pointer_buffer = [0; 8];
    LittleEndian::write_u64(&mut code_pointer_buffer, code_pointer);
    code[17] = code_pointer_buffer[0];
    code[18] = code_pointer_buffer[1];
    code[19] = code_pointer_buffer[2];
    code[20] = code_pointer_buffer[3];

    // 復帰場所を追記する
    // jmp命令で復帰す場所の相対アドレス位置を32バイト目に記述する
    // 今回はもともとのエントリーポイントのアドレスまでの相対位置を計算して記載する
    let offset = 0x00400000;  // メモリ配置時のオフセット
    let original_offset = e_entry - offset;
    let jmp_code = 31;  // jmp命令のある場所
    let code_size = program_size + jmp_code;
    let return_point = 0xff - (code_size as u64 - original_offset);
    let mut return_point_buffer = [0; 8];
    LittleEndian::write_u64(&mut return_point_buffer, return_point);
    code[31] = return_point_buffer[0];  // FIXME: 2bytes以上のfar jmpに対応する

    // 修正したコードを返す
    Vec::from(code)
}

fn inject(input: &[u8], code: &[u8]) -> Vec<u8> {
    let mut injected = Vec::from(input);

    // プログラム開始位置を注入したコードの場所にする
    // 24 ~ 31バイトがプログラムのエントリーポイントを記述する場所
    // メモリに展開されたあとのアドレスを記載することに注意
    let offset = 0x00400000;  // メモリ配置時のELFヘッダーオフセット
    let injected_entry = offset + input.len();
    let mut injected_entry_buffer = [0; 8];
    LittleEndian::write_u64(&mut injected_entry_buffer, injected_entry as u64);
    injected[24] = injected_entry_buffer[0];
    injected[25] = injected_entry_buffer[1];
    injected[26] = injected_entry_buffer[2];
    injected[27] = injected_entry_buffer[3];
    injected[28] = injected_entry_buffer[4];
    injected[29] = injected_entry_buffer[5];
    injected[30] = injected_entry_buffer[6];
    injected[31] = injected_entry_buffer[7];

    // 注入したコードを結合する
    injected.extend(code);

    injected
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value("injected.out"))]
    output: String,

    #[arg(value_names = ["INPUT"])]
    input: String,

    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    let input = read_file(&args.input);
    let e_entry = &input[24..32];
    let entry = LittleEndian::read_u64(&e_entry);  // TODO: ELFがBigEndianの場合に対応させる
    let code = injection_code(entry, input.len());
    let injected = inject(&input, &code);
    write_file(&args.output, &injected);

    if args.debug {
        println!("input: {}, output: {}", args.input, args.output);
        println!("e_entry: {:02x?}", &e_entry);
        println!("entry: {:08x?}", entry);
        println!("program: {:02x?}", &input);
        println!("code: {:02x?}", &code);
        println!("injected: {:02x?}", &injected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM: [u8; 240] = [
        0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x3e, 0x00, 0x01, 0x00, 0x00, 0x00, 0xb0, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00, 0x02, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0xbc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xbc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x48, 0xc7, 0xc0, 0x01, 0x00, 0x00, 0x00, 0x48, 0xc7, 0xc7, 0x01, 0x00, 0x00, 0x00, 0x48, 0xc7,
        0xc6, 0xe0, 0x00, 0x40, 0x00, 0x48, 0xc7, 0xc2, 0x0d, 0x00, 0x00, 0x00, 0x0f, 0x05, 0x48, 0xc7,
        0xc0, 0x3c, 0x00, 0x00, 0x00, 0x48, 0xc7, 0xc7, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x05, 0x00, 0x00,
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x57, 0x6f, 0x72, 0x6c, 0x64, 0x0a, 0x00, 0x00, 0x00, 0x00
    ];

    #[test]
    fn test() {
        let input = Vec::from(PROGRAM);

        let e_entry = &input[24..32];
        println!("e_entry: {:02x?}", &e_entry);
        assert_eq!(&e_entry, &[0xb0, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let entry = LittleEndian::read_u64(&e_entry);
        println!("e_entry: {:08x?}", entry);
        assert_eq!(entry, 0x004000b0);
        
        let code = injection_code(entry, input.len());
        println!("{:02x?}", &code);

        let injected = inject(&input, &code);
        println!("{:02x?}", &injected);
        assert_eq!(injected.len(), input.len() + code.len());
    }
}
