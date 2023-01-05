pub struct Level {
    pub id: &'static str,
    pub query: &'static str,
    pub website_name: &'static str,
    pub category_name: &'static str,
}

pub const LEVELS: [Level; 2] = [
    Level{
        id: "amazon-if",
        query: "website=\"amazon.com\" AND (
            SELECT COUNT(*) FROM categories WHERE categories.listing_id = listings.id AND categories.category = \"interesting-finds\"
        ) > 0",
        website_name: "Amazon",
        category_name: "Interesting Finds",
    },
    Level{
        id: "amazon-thi",
        query: "website = \"amazon.com\" AND (
            SELECT COUNT(*) FROM categories WHERE categories.listing_id = listings.id AND categories.category = \"hgg-hol-hi\"
        ) > 0",
        website_name: "Amazon",
        category_name: "Tools and Home Improvement",
    },
];

pub fn find_level(id: &str) -> Option<&'static Level> {
    for level in &LEVELS {
        if level.id == id {
            return Some(&level);
        }
    }
    None
}
