use super::dissector::TezosDissector;

#[rustfmt::skip]
use wireshark_epan_adapter::{
    Plugin, NameDescriptor, FieldDescriptor,
    PrefFilenameDescriptor,
    DissectorDescriptor,
};

#[no_mangle]
static plugin_version: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

#[no_mangle]
static plugin_want_major: i32 = 3;

#[no_mangle]
static plugin_want_minor: i32 = 2;

#[no_mangle]
extern "C" fn plugin_register() {
    if cfg!(debug_assertions) {
        let file = env!("PWD")
            .parse::<std::path::PathBuf>()
            .unwrap()
            .join("target/log.txt");
        simple_logging::log_to_file(file, log::LevelFilter::Info).unwrap();
    }

    Plugin::new(
        DissectorDescriptor {
            display_name: "Tezos\0",
            short_name: "tezos_tcp\0",
        },
        NameDescriptor {
            name: "Tezos Protocol\0",
            short_name: "tezos\0",
            filter_name: "tezos\0",
        },
        &[
            &[
                FieldDescriptor::Int64Dec {
                    name: "Chunk length\0",
                    abbrev: "tezos.chunk_length\0",
                },
                FieldDescriptor::String {
                    name: "Buffering incomplete chunk\0",
                    abbrev: "tezos.buffering\0",
                },
                FieldDescriptor::String {
                    name: "Conversation\0",
                    abbrev: "tezos.conversation_id\0",
                },
                FieldDescriptor::String {
                    name: "Connection message\0",
                    abbrev: "tezos.connection_msg\0",
                },
                FieldDescriptor::Int64Dec {
                    name: "Port\0",
                    abbrev: "tezos.connection_msg.port\0",
                },
                FieldDescriptor::String {
                    name: "Public key\0",
                    abbrev: "tezos.connection_msg.pk\0",
                },
                FieldDescriptor::String {
                    name: "Proof of work\0",
                    abbrev: "tezos.connection_msg.pow\0",
                },
                FieldDescriptor::String {
                    name: "Nonce\0",
                    abbrev: "tezos.connection_msg.nonce\0",
                },
                FieldDescriptor::String {
                    name: "Version\0",
                    abbrev: "tezos.connection_msg.version\0",
                },
                FieldDescriptor::String {
                    name: "MAC mismatch\0",
                    abbrev: "tezos.error\0",
                },
                FieldDescriptor::String {
                    name: "Identity required\0",
                    abbrev: "tezos.identity_required\0",
                },
                FieldDescriptor::String {
                    name: "Decrypted message\0",
                    abbrev: "tezos.decrypted_msg\0",
                },
            ],
        ],
        &[
            PrefFilenameDescriptor {
                name: "identity_json_file\0",
                title: "Identity JSON file\0",
                description: "JSON file with node identity information\0",
            },
        ],
    )
    .register(Box::new(TezosDissector::new()))
}
