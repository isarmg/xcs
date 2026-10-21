#[cfg(all(test, feature = "help", feature = "usage"))]
mod tests {
    use super::*;

    #[test]
    fn help_output_is_stable() {
        snapbox::assert_data_eq!(
            render_reference(cli()),
            snapbox::file!["snapshots/pacman.txt"].raw()
        );
    }

    fn render_reference(mut command: Command) -> String {
        command = command.term_width(0);
        command.build();

        let mut buffer = String::new();
        write_help(&mut buffer, &mut command, String::new());

        buffer
    }

    fn write_help(buffer: &mut String, cmd: &mut Command, mut path: String) {
        let header = if path.is_empty() { "#" } else { "##" };
        path.push(' ');
        path.push_str(cmd.get_name());

        buffer.push_str(header);
        buffer.push_str(&path);
        buffer.push_str("\n\n");
        buffer.push_str(&cmd.render_long_help().to_string());

        let has_generated_help = !cmd.is_disable_help_subcommand_set();
        for subcommand in cmd.get_subcommands_mut() {
            // Generated `help` nodes proxy other commands.
            if has_generated_help && subcommand.get_name() == "help" {
                continue;
            }

            buffer.push('\n');
            write_help(buffer, subcommand, path.clone());
        }
    }
}
