use time::data_serialization;

data_serialization::format_description!(); // unexpected end of input
data_serialization::format_description!("bad string", OffsetDateTime, "[year] [month]"); // module name is not ident
data_serialization::format_description!(my_format: OffsetDateTime, "[year] [month]"); // not a comma
data_serialization::format_description!(my_format,); // missing formattable and string
data_serialization::format_description!(my_format, "[year] [month]"); // missing formattable
data_serialization::format_description!(OffsetDateTime, "[year] [month]"); // missing ident
data_serialization::format_description!(my_format, OffsetDateTime); // missing string format
data_serialization::format_description!(my_format, OffsetDateTime,); // missing string format
data_serialization::format_description!(my_format, OffsetDateTime "[year] [month]"); // missing comma
data_serialization::format_description!(my_format, OffsetDateTime : "[year] [month]"); // not a comma
data_serialization::format_description!(my_format, OffsetDateTime, "[bad]"); // bad component name
data_serialization::format_description!(my_format, OffsetDateTime, not_string); // not in scope

fn main() {}
