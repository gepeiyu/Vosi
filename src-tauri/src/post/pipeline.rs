use super::hotword::HotwordReplacer;
use super::itn::apply_itn;

pub fn post_process(raw: &str, hotwords: &HotwordReplacer, hotword_enabled: bool) -> String {
    let mut text = raw.trim().to_string();
    if hotword_enabled {
        text = hotwords.apply(&text);
    }
    apply_itn(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_applies_itn_after_hotword() {
        let hotwords = HotwordReplacer::from_lines(vec![]);
        let result = post_process("一共一百二十三元", &hotwords, false);
        assert_eq!(result, "一共123元");
    }
}
