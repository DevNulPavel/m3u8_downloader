INSTALL_TOOLCHAINS:
	rustup target add aarch64-apple-darwin
	rustup target add x86_64-apple-darwin
	rustup target add x86_64-unknown-linux-gnu
	rustup target add x86_64-pc-windows-msvc

DEPLOY_LINUX_RELEASE:
	cargo build --release --target x86_64-unknown-linux-gnu && \
	cd target/x86_64-unknown-linux-gnu && \
	tar -czf m3u8_downloader-linux-x64.tar.gz m3u8_downloader

DEPLOY_WINDOWS_RELEASE:
	cargo build --release --target x86_64-pc-windows-msvc && \
	cd target/x86_64-pc-windows-msvc && \
	tar -czf m3u8_downloader-windows-x64.tar.gz m3u8_downloader

OSX_ARM64_RELEASE:
	cargo build --release --target aarch64-apple-darwin

OSX_X64_RELEASE:
	cargo build --release --target x86_64-apple-darwin

DEPLOY_OSX_UNIVERSAL_RELEASE: OSX_ARM64_RELEASE OSX_X64_RELEASE
	rm -rf target/osx_release_universal && \
	mkdir target/osx_release_universal && \
	lipo \
		-create \
			target/aarch64-apple-darwin/release/m3u8_downloader \
			target/x86_64-apple-darwin/release/m3u8_downloader \
		-output \
			target/osx_release_universal/m3u8_downloader && \
	cd target/osx_release_universal && \
	tar -czf m3u8_downloader-osx-universal.tar.gz m3u8_downloader && \
	shasum -a 256 m3u8_downloader-osx-universal.tar.gz



BUILD_DOC:
	cargo doc --open

TEST_PUBLISH:
	cargo publish --dry-run

PUBLISH:
	cargo publish