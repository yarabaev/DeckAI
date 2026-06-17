//! Smoke tests for `parse_chart`.

use slideglance_color::{ColorMap, ColorResolver, ColorScheme, Rgb};
use slideglance_model::{ChartType, LegendPosition};
use slideglance_parser::parse_chart;

fn test_resolver() -> ColorResolver {
    let scheme = ColorScheme {
        dk1: Rgb::new(0, 0, 0),
        lt1: Rgb::new(0xFF, 0xFF, 0xFF),
        dk2: Rgb::new(0, 0, 0),
        lt2: Rgb::new(0xFF, 0xFF, 0xFF),
        accent1: Rgb::new(0x44, 0x72, 0xC4),
        accent2: Rgb::new(0xED, 0x7D, 0x31),
        accent3: Rgb::new(0xA5, 0xA5, 0xA5),
        accent4: Rgb::new(0xFF, 0xC0, 0x00),
        accent5: Rgb::new(0x5B, 0x9B, 0xD5),
        accent6: Rgb::new(0x70, 0xAD, 0x47),
        hlink: Rgb::new(0, 0, 0),
        fol_hlink: Rgb::new(0, 0, 0),
    };
    ColorResolver::new(scheme, ColorMap::default())
}

#[test]
fn returns_none_for_empty_chart() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>"#;
    assert!(parse_chart(xml, &test_resolver()).unwrap().is_none());
}

#[test]
fn parses_minimal_bar_chart() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart>
            <c:plotArea>
                <c:barChart>
                    <c:barDir val="bar"/>
                    <c:ser>
                        <c:tx><c:v>Series 1</c:v></c:tx>
                        <c:val>
                            <c:numRef>
                                <c:numCache>
                                    <c:pt idx="0"><c:v>10</c:v></c:pt>
                                    <c:pt idx="1"><c:v>20</c:v></c:pt>
                                    <c:pt idx="2"><c:v>30</c:v></c:pt>
                                </c:numCache>
                            </c:numRef>
                        </c:val>
                        <c:cat>
                            <c:strRef>
                                <c:strCache>
                                    <c:pt idx="0"><c:v>A</c:v></c:pt>
                                    <c:pt idx="1"><c:v>B</c:v></c:pt>
                                    <c:pt idx="2"><c:v>C</c:v></c:pt>
                                </c:strCache>
                            </c:strRef>
                        </c:cat>
                    </c:ser>
                </c:barChart>
            </c:plotArea>
        </c:chart>
    </c:chartSpace>"#;
    let chart = parse_chart(xml, &test_resolver()).unwrap().unwrap();
    assert!(matches!(chart.chart_type, ChartType::Bar));
    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].name.as_deref(), Some("Series 1"));
    assert_eq!(chart.series[0].values, vec![10.0, 20.0, 30.0]);
    assert_eq!(chart.series[0].color.rgb, Rgb::new(0x44, 0x72, 0xC4));
    assert_eq!(chart.categories, vec!["A", "B", "C"]);
}

#[test]
fn parses_chart_legend_position() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart>
            <c:plotArea>
                <c:lineChart>
                    <c:ser/>
                </c:lineChart>
            </c:plotArea>
            <c:legend><c:legendPos val="r"/></c:legend>
        </c:chart>
    </c:chartSpace>"#;
    let chart = parse_chart(xml, &test_resolver()).unwrap().unwrap();
    assert!(matches!(chart.legend.unwrap().position, LegendPosition::R));
}

#[test]
fn detects_combo_chart() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart>
            <c:plotArea>
                <c:barChart><c:ser/></c:barChart>
                <c:lineChart><c:ser/></c:lineChart>
            </c:plotArea>
        </c:chart>
    </c:chartSpace>"#;
    let chart = parse_chart(xml, &test_resolver()).unwrap().unwrap();
    assert!(chart.is_combo);
    assert_eq!(chart.series.len(), 2);
    assert!(matches!(
        chart.series[0].sub_chart_type,
        Some(ChartType::Bar)
    ));
    assert!(matches!(
        chart.series[1].sub_chart_type,
        Some(ChartType::Line)
    ));
}

#[test]
fn parses_chart_title() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
                                xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <c:chart>
            <c:title>
                <c:tx><c:rich>
                    <a:p><a:r><a:t>Sales</a:t></a:r></a:p>
                </c:rich></c:tx>
            </c:title>
            <c:plotArea>
                <c:pieChart><c:ser/></c:pieChart>
            </c:plotArea>
        </c:chart>
    </c:chartSpace>"#;
    let chart = parse_chart(xml, &test_resolver()).unwrap().unwrap();
    assert_eq!(chart.title.as_deref(), Some("Sales"));
    assert!(matches!(chart.chart_type, ChartType::Pie));
}

#[test]
fn doughnut_default_hole_size_50() {
    let xml = r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
        <c:chart><c:plotArea>
            <c:doughnutChart><c:ser/></c:doughnutChart>
        </c:plotArea></c:chart>
    </c:chartSpace>"#;
    let chart = parse_chart(xml, &test_resolver()).unwrap().unwrap();
    assert!((chart.hole_size.unwrap() - 50.0).abs() < 1e-9);
}
