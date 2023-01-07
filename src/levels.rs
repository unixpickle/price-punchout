pub struct Level {
    pub id: &'static str,
    pub website_name: &'static str,
    pub category_name: &'static str,
    query: &'static str,
}

pub const LEVELS: [Level; 2] = [
    Level{
        id: "amazon-if",
        website_name: "Amazon",
        category_name: "Interesting Finds",
        query: "website=\"amazon.com\" AND (
            SELECT COUNT(*) FROM categories WHERE categories.listing_id = listings.id AND categories.category = \"interesting-finds\"
        ) > 0",
    },
    Level{
        id: "amazon-thi",
        website_name: "Amazon",
        category_name: "Tools and Home Improvement",
        query: "website=\"amazon.com\" AND (
            SELECT COUNT(*) FROM categories WHERE categories.listing_id = listings.id AND categories.category = \"interesting-finds\"
        ) > 0",
    },
];

impl Level {
    pub fn find_by_id(id: &str) -> Option<&'static Level> {
        LEVELS.iter().filter(|x| x.id == id).next()
    }

    pub fn listing_query(&self) -> &'static str {
        self.query
    }
}
