# Author: Bill Doughty
# Version: 0.1

from prompt_toolkit.completion import Completer, Completion
import re
from string_space_client import StringSpaceClient

class StringSpaceCompleter(Completer):
    def __init__(self, **kwargs):
        port = kwargs.get('port', 7878)
        host = kwargs.get('host', '127.0.0.1')
        debug = kwargs.get('debug', False)
        self.client = StringSpaceClient(host, port, debug)

    def get_completions(self, document, complete_event):
        word_before_cursor = document.get_word_before_cursor(WORD=True).lower()

        if len(word_before_cursor) < 2 and not complete_event.completion_requested:
            return

        # if word_before_cursor ends with a non-word character, return
        if re.search(r'[^\w_\-\s]', word_before_cursor):
            return

        doc_words = [word.lower() for word in self.parse_text(document.text)]
        # get unique doc_words
        doc_words = list(set(doc_words))

        # remove word_before_cursor from doc_words if it exists
        if word_before_cursor in doc_words:
            doc_words.remove(word_before_cursor)

        # filter only words that start with the same letter as word_before_cursor
        doc_words = [word for word in doc_words if word.lower().startswith(word_before_cursor.lower())]
        completion_suggestions = self.client.prefix_search(word_before_cursor)
        spell_suggestions = self.client.similar_search(word_before_cursor, threshold=0.6)

        # combine doc_words and completion_suggestions and spell_suggestions into a single list
        suggestions = doc_words + completion_suggestions + spell_suggestions

        # remove duplicates in suggestions while preserving order
        seen = set()
        result = []
        word_in_suggestions = word_before_cursor in suggestions
        for word in suggestions:
            if word not in seen and word != word_before_cursor:
                seen.add(word)
                result.append(word)
        if word_in_suggestions:
            # insert word_before_cursor at the beginning of the list
            result.insert(0, word_before_cursor)
        suggestions = result
        for suggestion in suggestions:
            if suggestion.strip() != '':
                yield Completion(suggestion, start_position=-len(word_before_cursor))

    def stop(self):
        self.client.disconnect()

    def parse_text(self, text: str) -> list[str]:
        # split the text into words by regex /s+/
        words = re.split(r'[^\w_\-s]+', text)
        # get unique words
        words = list(set(words))
        # filter out words that are too short or too long
        words = [word for word in words if len(word) >= 3]
        return words

    def add_words_from_text(self, text):
        words = self.parse_text(text)
        self.client.add_words(words)

    def add_words(self, words):
        self.client.add_words(words)
