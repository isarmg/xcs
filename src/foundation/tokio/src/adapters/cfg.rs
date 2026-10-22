macro_rules! cfg_adapters_codec {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapters-codec")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapters-codec")))]
            $item
        )*
    }
}

macro_rules! cfg_adapters_compat {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapters-compat")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapters-compat")))]
            $item
        )*
    }
}

macro_rules! cfg_adapters_net {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "adapters-net", feature = "adapters-codec"))]
            #[cfg_attr(docsrs, doc(cfg(all(feature = "adapters-net", feature = "adapters-codec"))))]
            $item
        )*
    }
}

macro_rules! cfg_adapters_io {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapters-io")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapters-io")))]
            $item
        )*
    }
}

cfg_adapters_io! {
    macro_rules! cfg_adapters_io_util {
        ($($item:item)*) => {
            $(
                #[cfg(feature = "adapters-io-util")]
                #[cfg_attr(docsrs, doc(cfg(feature = "adapters-io-util")))]
                $item
            )*
        }
    }
}

macro_rules! cfg_adapters_rt {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapters-rt")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapters-rt")))]
            $item
        )*
    }
}

macro_rules! cfg_adapters_not_rt {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "adapters-rt"))]
            $item
        )*
    }
}

macro_rules! cfg_adapters_time {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "adapters-time")]
            #[cfg_attr(docsrs, doc(cfg(feature = "adapters-time")))]
            $item
        )*
    }
}
