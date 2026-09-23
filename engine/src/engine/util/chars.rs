pub trait StrParsingExtensions {
    fn as_char_array<const EXPECTED_SIZE: usize>(&self) -> Option<[char; EXPECTED_SIZE]>;
}

impl StrParsingExtensions for str {
    fn as_char_array<const EXPECTED_SIZE: usize>(&self) -> Option<[char; EXPECTED_SIZE]> {
        let mut chars = self.chars();

        let arr: [Option<char>; EXPECTED_SIZE] = std::array::from_fn(|_| chars.next());

        if arr.contains(&None) || chars.next().is_some() {
            return None;
        }

        Some(arr.map(Option::unwrap))
    }
}
