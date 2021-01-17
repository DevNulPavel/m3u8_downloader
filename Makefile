INSTALL_TOOLCHAINS:
	rustup target add aarch64-apple-darwin
	rustup target add x86_64-apple-darwin

ARM64_RELEASE:
	cargo build --release --target aarch64-apple-darwin

X64_RELEASE:
	cargo build --release --target x86_64-apple-darwin

DEPLOY: ARM64_RELEASE X64_RELEASE
	rm -rf target/release_universal && \
	mkdir target/release_universal && \
	lipo \
		-create \
			target/aarch64-apple-darwin/release/m3u8_downloader \
			target/x86_64-apple-darwin/release/m3u8_downloader \
		-output \
			target/release_universal/m3u8_downloader && \
	cd target/release_universal && \
	tar -czf m3u8_downloader-universal.tar.gz m3u8_downloader && \
	shasum -a 256 m3u8_downloader-universal.tar.gz

AUDID:
	brew audit --strict --online m3u8_downloader