use itertools::Itertools;

use crate::anthropic::UserSegment;

pub fn serialize_segments(segments: &[UserSegment]) -> String {
    let inner = segments
        .iter()
        .map(|s| match s {
            UserSegment::Text(t) => format!("  <text>{}</text>", t),
            UserSegment::Code(c) => format!("  <code>{}</code>", c),
            UserSegment::ToolResult(r) => format!("  <tool_result>{}</tool_result>", r),
        })
        .join("\n");

    format!("<message>\n{}\n</message>", inner)
}
