ELF injector
=====


ELFファイルのエントリポイントを上書きして任意のコードを埋め込む
埋め込んだコードをエントリポイントに設定し処理の戻り先として元のコードを指定する


実行
-----
```sh
cargo run -- -o <output> <elffile>
```


結果
-----
ubuntuやdockerコンテナ内を想定しています

元の結果
```sh
cd example
xxd -r helloworld > helloworld.out
chmod +x helloworld.out
./helloworld.out
# Hello,World
```

実行
```sh
cd example
cargo run -- -o ./injected.out ./helloworld.out
```

実行後
```sh
cd example
chmod +x ./injected.out
./injected.out
# Injected\nHello,World
```


備考
-----
書き換えるエントリポイント
```
00000000: 7f45 4c46 0201 0100 0000 0000 0000 0000
00000010: 0200 3e00 0100 0000 xxxx xxxx xxxx xxxx
```

埋め込むコード
```
48c7 c001 0000 0048 c7c7 0100 0000 48c7  // write(1, "Injected\n", 9) 標準出力に "Injected\n" の文字列から9文字出力する
c6xx xxxx xx48 c7c2 0900 0000 0f05 ebxx  // jmp xx(元のエントリポイント-追加コード) の処理に戻る
496e 6a65 6374 6564 0a00 0000 0000 0000  // Injected\n の文字列
```


サンプルコードについて
-----
以下のようなHello,Worldを出力するプログラム

それぞれ `as -o <output> <file.s>` や `gcc -c -o <output> <file.c>` などで
glibcなどがリンクされる前のものを出力し `xxd` や `objdump -d` などで表示されたアセンブリを直接ELFから呼び出されるようにしています

アセンブリ
```asm
.globl _start
_start:
    /* write(1, msg, 13) */
    mov $1,%rax
    mov $1,%rdi
    mov $msg,%rsi
    mov $13,%rdx
    syscall

    /* exit(0) */
    mov $60,%rax
    mov $0,%rdi
    syscall

.data
    msg: .asciz "Hello,World\n"

```

C
```c
int main() {
  // 1は標準出力
  // 13は"Hello,World\n"の文字数
	write(1, "Hello,World\n", 13);
	return 0;
}
```


参考
-----
- [最小限のELF](https://keens.github.io/blog/2020/04/12/saishougennoelf/)

