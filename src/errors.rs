error_chain! {
    foreign_links {
        Io(::std::io::Error);
        ConfigParse(::toml::de::Error);
        AddrParse(::std::net::AddrParseError);
        Log(::log::SetLoggerError);
        Http(::hyper::Error);
        Regex(::regex::Error);
    }

    errors {
        InvalidRoute(r: String) {
            description("invalid route specified")
            display("invalid route specified: '{}'", r)
        }
    }
}
