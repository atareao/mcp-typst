pub mod compile;
pub mod compile_template;
pub mod export;
pub mod list_fonts;
pub mod list_templates;
pub mod query;

pub use compile::Compile;
pub use compile_template::CompileTemplate;
pub use export::Export;
pub use list_fonts::ListFonts;
pub use list_templates::ListTemplates;
pub use query::Query;

use rust_mcp_sdk::tool_box;

tool_box!(
    TypstTools,
    [
        Compile,
        ListTemplates,
        CompileTemplate,
        Export,
        Query,
        ListFonts
    ]
);
