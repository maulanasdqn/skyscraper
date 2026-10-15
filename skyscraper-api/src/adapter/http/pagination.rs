use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

impl PaginationMeta {
    pub fn new(page: i64, per_page: i64, total: i64) -> Self {
        let total_pages = if per_page > 0 {
            (total + per_page - 1) / per_page
        } else {
            0
        };
        Self {
            page,
            per_page,
            total,
            total_pages,
        }
    }
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, total: i64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(page, per_page, total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_meta_calculates_pages() {
        let meta = PaginationMeta::new(1, 20, 55);
        assert_eq!(meta.total_pages, 3);
    }

    #[test]
    fn pagination_meta_exact_division() {
        let meta = PaginationMeta::new(1, 10, 30);
        assert_eq!(meta.total_pages, 3);
    }

    #[test]
    fn pagination_meta_zero_total() {
        let meta = PaginationMeta::new(1, 20, 0);
        assert_eq!(meta.total_pages, 0);
    }

    #[test]
    fn paginated_response_serializes() {
        let resp = PaginatedResponse::new(vec![1, 2, 3], 1, 20, 3);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["data"], serde_json::json!([1, 2, 3]));
        assert_eq!(json["pagination"]["page"], 1);
        assert_eq!(json["pagination"]["total"], 3);
        assert_eq!(json["pagination"]["total_pages"], 1);
    }
}
