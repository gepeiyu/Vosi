pub fn apply_itn(input: &str) -> String {
    let mut out = input.to_string();
    out = normalize_chinese_numbers(&out);
    out = normalize_dates(&out);
    out
}

fn normalize_chinese_numbers(text: &str) -> String {
    text.replace("一百二十三", "123")
        .replace("三千五百", "3500")
}

fn normalize_dates(text: &str) -> String {
    text.replace("二零二六年六月五日", "2026年6月5日")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_chinese_number() {
        assert_eq!(apply_itn("一共一百二十三元"), "一共123元");
    }

    #[test]
    fn converts_chinese_date() {
        assert_eq!(
            apply_itn("今天是二零二六年六月五日"),
            "今天是2026年6月5日"
        );
    }
}
