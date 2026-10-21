#[cfg(test)]
mod test {
    use super::{DiskStat, disk_stats_inner};
    use std::collections::HashMap;

    #[test]
    fn test_disk_stat_parsing() {
        // Content of a (very nicely formatted) `/proc/diskstats` file.
        let file_content = "\
 259       0 nvme0n1   571695 101559 38943220 165643 9824246  1076193 462375378 4140037  0  1038904 4740493  254020 0  1436922320 68519 306875 366293
 259       1 nvme0n1p1 240    2360   15468    48     2        0       2         0        0  21      50       8      0  2373552    2     0      0
 259       2 nvme0n1p2 243    10     11626    26     63       39      616       125      0  84      163      44     0  1075280    11    0      0
 259       3 nvme0n1p3 571069 99189  38910302 165547 9824180  1076154 462374760 4139911  0  1084855 4373964  253968 0  1433473488 68505 0      0
 253       0 dm-0      670206 0      38909056 259490 10900330 0       462374760 12906518 0  1177098 13195902 253968 0  1433473488 29894 0      0
 252       0 zram0     2382   0      20984    11     260261   0       2082088   2063     0  1964    2074     0      0  0          0     0      0
 1         2 bla       4      5      6        7      8        9       10        11       12 13      14       15     16 17         18    19     20
";

        let data = disk_stats_inner(file_content);
        let expected_data: HashMap<String, DiskStat> = HashMap::from([
            (
                "nvme0n1".to_string(),
                DiskStat {
                    sectors_read: 38943220,
                    sectors_written: 462375378,
                },
            ),
            (
                "nvme0n1p1".to_string(),
                DiskStat {
                    sectors_read: 15468,
                    sectors_written: 2,
                },
            ),
            (
                "nvme0n1p2".to_string(),
                DiskStat {
                    sectors_read: 11626,
                    sectors_written: 616,
                },
            ),
            (
                "nvme0n1p3".to_string(),
                DiskStat {
                    sectors_read: 38910302,
                    sectors_written: 462374760,
                },
            ),
            (
                "dm-0".to_string(),
                DiskStat {
                    sectors_read: 38909056,
                    sectors_written: 462374760,
                },
            ),
            (
                "zram0".to_string(),
                DiskStat {
                    sectors_read: 20984,
                    sectors_written: 2082088,
                },
            ),
            // This one ensures that we read the correct fields.
            (
                "bla".to_string(),
                DiskStat {
                    sectors_read: 6,
                    sectors_written: 10,
                },
            ),
        ]);

        assert_eq!(data, expected_data);
    }

    #[test]
    fn disk_entry_with_less_information() {
        let file_content = "\
 systemd-1      /efi autofs rw,relatime,fd=181,pgrp=1,timeout=120,minproto=5,maxproto=5,direct,pipe_ino=8311 0 0
 /dev/nvme0n1p1 /efi vfat   rw,nosuid,nodev,noexec,relatime,nosymfollow,fmask=0077,dmask=0077                0 0
";

        let data = disk_stats_inner(file_content);
        let expected_data: HashMap<String, DiskStat> = HashMap::from([
            (
                "autofs".to_string(),
                DiskStat {
                    sectors_read: 0,
                    sectors_written: 0,
                },
            ),
            (
                "vfat".to_string(),
                DiskStat {
                    sectors_read: 0,
                    sectors_written: 0,
                },
            ),
        ]);

        assert_eq!(data, expected_data);
    }
}
