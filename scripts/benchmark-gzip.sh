#!/bin/bash
set -e;

# make build-sandbox
sudo cp sandbox-container/build/rootfs.tar /tmp/rootfs.tar
sudo chown $USER:$USER /tmp/rootfs.tar
du -h /tmp/rootfs.tar

# For different ZSTD rates {1, 6, 12} measure the time it takes to compress and decompress the rootfs.tar, as well how big it is
for i in {1,6,12}
do
  echo "-------------"
  echo "Compressing w/ ZSTD rate: $i"
  time zstd -T0 -"$i" -qf /tmp/rootfs.tar
  du -h /tmp/rootfs.tar.zst
  echo "-------------"
  echo "Decompressing..."
  time zstd -qdf /tmp/rootfs.tar.zst
  du -h /tmp/rootfs.tar
done

# For different GZIP rates (1, 3, 6, 9) measure the time it takes to compress and decompress the rootfs.tar, as well how big it is
for i in {1,3,6,9}
do
  echo "-------------"
  echo "Compressing w/ GZIP rate: $i"
  time pigz -f -"$i" /tmp/rootfs.tar
  du -h /tmp/rootfs.tar.gz
  echo "-------------"
  echo "Decompressing..."
  time pigz -df /tmp/rootfs.tar.gz
  du -h /tmp/rootfs.tar
done

rm /tmp/rootfs.tar.zst
