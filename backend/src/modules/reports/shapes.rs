#[derive(Clone, Copy)]
pub enum Geometry {
    Polygon(&'static str),
    Path(&'static str),
    Ellipse,
}

pub const DEFAULT: &str = "rect";

pub const KINDS: &[&str] = &[
    "rect",
    "ellipse",
    "triangle",
    "right-triangle",
    "diamond",
    "parallelogram",
    "trapezoid",
    "pentagon",
    "hexagon",
    "star",
    "arrow",
    "plus",
    "heart",
];

pub fn normalize(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if KINDS.contains(&value.as_str()) {
        value
    } else {
        DEFAULT.to_string()
    }
}

pub fn geometry(shape: &str) -> Geometry {
    match shape {
        "ellipse" => Geometry::Ellipse,
        "heart" => Geometry::Path(
            "M50 88 C20 68 4 50 4 30 C4 14 16 4 30 4 C39 4 46 9 50 16 C54 9 61 4 70 4 C84 4 96 14 96 30 C96 50 80 68 50 88 Z",
        ),
        "triangle" => Geometry::Polygon("50,2 98,98 2,98"),
        "right-triangle" => Geometry::Polygon("2,2 98,98 2,98"),
        "diamond" => Geometry::Polygon("50,1 99,50 50,99 1,50"),
        "parallelogram" => Geometry::Polygon("25,2 100,2 75,98 0,98"),
        "trapezoid" => Geometry::Polygon("22,4 78,4 99,96 1,96"),
        "pentagon" => Geometry::Polygon("50,2 95.65,35.16 78.2,88.83 21.8,88.83 4.35,35.16"),
        "hexagon" => Geometry::Polygon("50,2 91.6,26 91.6,74 50,98 8.4,74 8.4,26"),
        "star" => Geometry::Polygon(
            "50,0 61.81,38.2 100,38.2 69.1,61.8 80.92,100 50,76.4 19.09,100 30.9,61.8 0,38.2 38.19,38.2",
        ),
        "arrow" => Geometry::Polygon("0,30 60,30 60,8 100,50 60,92 60,70 0,70"),
        "plus" => Geometry::Polygon(
            "35,0 65,0 65,35 100,35 100,65 65,65 65,100 35,100 35,65 0,65 0,35 35,35",
        ),
        _ => Geometry::Polygon("0,0 100,0 100,100 0,100"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_known_and_unknown_shapes() {
        assert_eq!(normalize("Star"), "star");
        assert_eq!(normalize(" hexagon "), "hexagon");
        assert_eq!(normalize("rect"), "rect");
        assert_eq!(normalize("blob"), DEFAULT);
        assert_eq!(normalize(""), DEFAULT);
        assert!(matches!(geometry("star"), Geometry::Polygon(_)));
        assert!(matches!(geometry("heart"), Geometry::Path(_)));
        assert!(matches!(geometry("ellipse"), Geometry::Ellipse));
    }
}
