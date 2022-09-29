## 介绍

个人用途Bot:

目前支持的功能有：

1. 添加/随机图片(不支持回复添加,貌似是协议的问题)
2. 在线ghci功能(目前无docker支持)

## 目前在的群

- 寄のRust/C/C++/Java编程交流
- 后现代魔法小组
- 泛型scheme异步歧想水电站
- 人才辈出委员会
- .etc

## 使用方法

运行

```bash
cargo build --release
```

之后在`target/release`里找到qq_bot可执行文件

将qq_bot放到你喜欢的地方，运行

```bash
chmod +x qq_bot #提权，若有此权限不用执行
./qq_bot --uin QQ号 --password 密码
```

第一次运行需拿手机扫码或滑块

关于滑块请遵循proc_qq的步骤

## 开源协议

根据AGPL协议开源(因为`ricq`也是AGPL协议)

Note: 最近好像要更改至MPL or MIT，到时候也会跟着改变

## 作者

- Caviar-X(maintainer,creator)

- spore(help,optimize & prettify code)

  