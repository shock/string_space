pub(crate) struct WordStruct {
    word: String,
    count: u32,
}

impl WordStruct {
    pub(crate) fn new(word: String, count: u32) -> WordStruct {
        WordStruct {
            word,
            count,
        }
    }

    pub(crate) fn get_word(&self) -> &String {
        &self.word
    }

    pub(crate) fn get_count(&self) -> u32 {
        self.count
    }
}

impl Clone for WordStruct {
    fn clone(&self) -> Self {
        WordStruct::new(self.word.clone(), self.count)
    }
}

impl PartialEq for WordStruct {
    fn eq(&self, other: &Self) -> bool {
        self.word == other.word
    }
}

impl Eq for WordStruct {}

impl PartialOrd for WordStruct {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WordStruct {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.word.cmp(&other.word)
    }
}