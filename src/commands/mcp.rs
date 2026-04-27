use anyhow::Result;

use crate::{cli::McpServeArgs, mcp};

pub fn serve(args: McpServeArgs) -> Result<()> {
    mcp::serve_stdio(&args.target)
}
