//! The examples of limu's gallery (internal/gallery/examples in limu's
//! repository, under its MIT license), a file for each type of diagram:
//! front matter with the type's title, then each example as a heading and a
//! Mermaid code block.

/// The files of the gallery.
const FILES: [&str; 32] = [
    include_str!("diagram_samples/agentflow.md"),
    include_str!("diagram_samples/architecture.md"),
    include_str!("diagram_samples/block.md"),
    include_str!("diagram_samples/c4.md"),
    include_str!("diagram_samples/class.md"),
    include_str!("diagram_samples/cynefin.md"),
    include_str!("diagram_samples/er.md"),
    include_str!("diagram_samples/eventmodeling.md"),
    include_str!("diagram_samples/flowchart.md"),
    include_str!("diagram_samples/gantt.md"),
    include_str!("diagram_samples/gitGraph.md"),
    include_str!("diagram_samples/ishikawa.md"),
    include_str!("diagram_samples/journey.md"),
    include_str!("diagram_samples/kanban.md"),
    include_str!("diagram_samples/mindmap.md"),
    include_str!("diagram_samples/packet.md"),
    include_str!("diagram_samples/pie.md"),
    include_str!("diagram_samples/quadrantChart.md"),
    include_str!("diagram_samples/radar.md"),
    include_str!("diagram_samples/railroad.md"),
    include_str!("diagram_samples/requirement.md"),
    include_str!("diagram_samples/sankey.md"),
    include_str!("diagram_samples/sequence.md"),
    include_str!("diagram_samples/state.md"),
    include_str!("diagram_samples/swimlane.md"),
    include_str!("diagram_samples/timeline.md"),
    include_str!("diagram_samples/treeView.md"),
    include_str!("diagram_samples/treemap.md"),
    include_str!("diagram_samples/usecase.md"),
    include_str!("diagram_samples/venn.md"),
    include_str!("diagram_samples/wardley.md"),
    include_str!("diagram_samples/xychart.md"),
];

/// A type of diagram and its examples.
pub struct SampleType {
    pub title: String,
    pub examples: Vec<Sample>,
}

/// An example's title and its Mermaid source.
pub struct Sample {
    pub title: String,
    pub src: String,
}

/// The types of diagram with their examples, by title.
pub fn samples() -> Vec<SampleType> {
    let mut types: Vec<SampleType> = FILES.iter().map(|text| parse(text)).collect();
    types.sort_by(|a, b| a.title.cmp(&b.title));
    types
}

/// Reads a file of the gallery: the title in its front matter, and each
/// heading with the code block under it.
fn parse(text: &str) -> SampleType {
    let mut t = SampleType {
        title: String::new(),
        examples: Vec::new(),
    };
    let mut front = text.starts_with("---\n");
    let mut title = "";
    let mut code: Vec<&str> = Vec::new();
    let mut in_code = false;
    for (i, line) in text.split('\n').enumerate() {
        if front {
            if i > 0 && line == "---" {
                front = false;
            } else if let Some(v) = line.strip_prefix("title: ") {
                t.title = v.to_string();
            }
        } else if in_code {
            if line == "```" {
                t.examples.push(Sample {
                    title: title.to_string(),
                    src: code.join("\n"),
                });
                in_code = false;
                code.clear();
            } else {
                code.push(line);
            }
        } else if line == "```mermaid" {
            in_code = true;
        } else if let Some(v) = line.strip_prefix("## ") {
            title = v;
        }
    }
    t
}
