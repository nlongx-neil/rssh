CUR_DIR=$(cd $(dirname $0);pwd)
cd $CUR_DIR

cargo build --target x86_64-unknown-linux-musl --release

if [ $? -ne 0 ];then
    echo "Build failed!"
    exit 1
fi

if [ -f "rssh" ];then
    rm rssh
fi

cp $CUR_DIR/target/x86_64-unknown-linux-musl/release/rssh rssh
/bin/upx --best --lzma rssh