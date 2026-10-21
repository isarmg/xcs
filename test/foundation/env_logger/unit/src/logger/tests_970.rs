#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_get_filter_reads_from_var_if_set() {
        env::set_var("env_get_filter_reads_from_var_if_set", "from var");

        let env = Env::new().filter_or("env_get_filter_reads_from_var_if_set", "from default");

        assert_eq!(Some("from var".to_owned()), env.get_filter());
    }

    #[test]
    fn env_get_filter_reads_from_default_if_var_not_set() {
        env::remove_var("env_get_filter_reads_from_default_if_var_not_set");

        let env = Env::new().filter_or(
            "env_get_filter_reads_from_default_if_var_not_set",
            "from default",
        );

        assert_eq!(Some("from default".to_owned()), env.get_filter());
    }

    #[test]
    fn env_get_write_style_reads_from_var_if_set() {
        env::set_var("env_get_write_style_reads_from_var_if_set", "from var");

        let env =
            Env::new().write_style_or("env_get_write_style_reads_from_var_if_set", "from default");

        assert_eq!(Some("from var".to_owned()), env.get_write_style());
    }

    #[test]
    fn env_get_write_style_reads_from_default_if_var_not_set() {
        env::remove_var("env_get_write_style_reads_from_default_if_var_not_set");

        let env = Env::new().write_style_or(
            "env_get_write_style_reads_from_default_if_var_not_set",
            "from default",
        );

        assert_eq!(Some("from default".to_owned()), env.get_write_style());
    }

    #[test]
    fn builder_parse_env_overrides_existing_filters() {
        env::set_var(
            "builder_parse_default_env_overrides_existing_filters",
            "debug",
        );
        let env = Env::new().filter("builder_parse_default_env_overrides_existing_filters");

        let mut builder = Builder::new();
        builder.filter_level(LevelFilter::Trace);
        // Overrides global level to debug
        builder.parse_env(env);

        assert_eq!(builder.filter.build().filter(), LevelFilter::Debug);
    }
}
