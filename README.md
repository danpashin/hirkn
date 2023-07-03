# HiRKN

## Description
This is a small OpenWRT utility that works as an auto-updater for RKN blocked IPs and subnets for NFTables. I created it personally for myself so feel free to open an issue if utility doesn't have features you need.

## Current features
* Works with NFTables which are default on OpenWRT 22.03
* Supports multiple sets with multiple URLs in each one
* Auto-updates sets by schedule

## Building
1. Download OpenWRT sources, select your device and packages by using `make menuconfig`
2. Edit your feeds. They should include my source repo:
    ```
    $ nano feeds.conf	
    src-git danpashin-openwrt https://github.com/danpashin/openwrt-feed.git
    src-git .....
    src-git .....
	```
3. Build all firmware by just typing `make -j $(nproc)` or just hirkn: `make package/hirkn/compile -j $(nproc)`

## License

HiRKN is licensed under MIT license

See [LICENSE](LICENSE) for the full text.
