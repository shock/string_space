# Author: Bill Doughty
# Version: 0.1

from prompt_toolkit.completion import Completer, Completion
import re
from string_space_client import StringSpaceClient, ProtocolError
import sys

class StringSpaceCompleter(Completer):
    def __init__(self, **kwargs):
        port = kwargs.get('port', 7878)
        host = kwargs.get('host', '127.0.0.1')
        debug = kwargs.get('debug', False)
        self.disabled = False
        self.client = StringSpaceClient(host, port, debug)
        try:
            self.client.connect()
        except ProtocolError as e:
            print(f"WARNING: {e}")
            print("StringSpaceCompleter is disabled.  Launch StringSpaceServer and restart this app for word completion suggestions.")
            self.disabled = True

    def get_completions(self, document, complete_event):
        if self.disabled:
            return
        # word_before_cursor = document.get_word_before_cursor(WORD=True).lower()
        word_before_cursor = document.get_word_before_cursor(WORD=True)

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
        # completion_suggestions = self.client.fuzzy_subsequence_search(word_before_cursor)
        # print("Completion suggestions:\n", completion_suggestions, file=sys.stderr, flush=True)
        # spell_suggestions = self.client.similar_search(word_before_cursor, threshold=0.6)
        # print("Spell suggestions:\n", spell_suggestions, file=sys.stderr, flush=True)
        # # combine completion_suggestions and spell_suggestions into a single list
        # suggestions = completion_suggestions + spell_suggestions
        suggestions = self.client.best_completions_search(word_before_cursor, limit=10)
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
        # # for any suggestions that are in doc_words, move them to the front of the list
        # doc_sorted_suggestions = [word for word in result if word in doc_words]
        # other_suggestions = [word for word in result if word not in doc_words]
        # suggestions = doc_sorted_suggestions + other_suggestions
        for suggestion in suggestions:
            if suggestion.strip() != '':
                yield Completion(suggestion, start_position=-len(word_before_cursor))

    def stop(self):
        if self.disabled:
            return
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
        if self.disabled:
            return
        words = self.parse_text(text)
        self.client.add_words(words)

    def add_words(self, words):
        if self.disabled:
            return
        self.client.add_words(words)
