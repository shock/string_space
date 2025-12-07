# Author: Bill Doughty
# Version: 0.9

import time
from prompt_toolkit.completion import Completer, Completion, CompleteEvent
from prompt_toolkit.document import Document

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
        self.last_completion_time = time.time()
        # Don't connect here - let StringSpaceClient handle connections per request
        # This avoids connection state issues with concurrent access
        try:
            # Just test if server is reachable with a simple request
            # Use a timeout to avoid hanging if server is down
            import socket
            test_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            test_sock.settimeout(2.0)  # 2 second timeout
            test_sock.connect((host, port))
            test_sock.close()
        except (ConnectionRefusedError, socket.timeout, OSError) as e:
            print(f"WARNING: Cannot connect to StringSpaceServer on {host}:{port}")
            print(f"Error: {e}")
            print("StringSpaceCompleter is disabled. Launch StringSpaceServer and restart this app for word completion suggestions.")
            self.disabled = True

    def get_completions(self, document: Document, complete_event: CompleteEvent):
        if self.disabled:
            return
        now = time.time()
        # get delta since last completion
        delta = now - self.last_completion_time
        # if delta is less than 100 milliseconds, return
        if delta < 0.1:
            return
        # Don't update timestamp yet - wait until after network call completes
        # self.last_completion_time = now  # REMOVED - will update after network call
        word_before_cursor = document.get_word_before_cursor(WORD=True)

        if len(word_before_cursor) < 2 and not complete_event.completion_requested:
            return

        # if word_before_cursor ends with a non-word character, return
        if re.search(r'[^\w_\-\_\']', word_before_cursor):
            return

        # remove starting non-word characters from word_before_cursor
        while re.match(r'^[^\w_\-\']', word_before_cursor):
            word_before_cursor = word_before_cursor[1:]

        suggestions = self.client.best_completions_search(word_before_cursor, limit=10)
        
        # Update timestamp AFTER network call completes
        self.last_completion_time = time.time()

        # for each suggestion in suggestions, if the first character is lower case and matches the first character of word_before_cursor, change it to upper case
        for i in range(len(suggestions)):
            if (suggestions[i][0].islower() and word_before_cursor[0].isupper() and
                suggestions[i][0].lower() == word_before_cursor[0].lower()):
                suggestions[i] = word_before_cursor[0] + suggestions[i][1:]

        for suggestion in suggestions:
            if suggestion.strip() != '':
                yield Completion(suggestion, start_position=-len(word_before_cursor))

    def stop(self):
        if self.disabled:
            return
        self.client.disconnect()

    def parse_text(self, text: str) -> list[str]:
        # split the text into words by regex /s+/
        words = re.split(r'[^\w_\-\']+', text)
        # get unique words
        words = list(set(words))
        # filter out words that are too short or too long
        words = [word for word in words if len(word) >= 3]
        #filter out words beginning with "'"
        words = [word for word in words if not word.startswith("'")]
        return words

    def add_words_from_text(self, text: str):
        if self.disabled:
            return
        
        words = self.parse_text(text)
        if len(words) == 0:
            return
        self.client.add_words(words)

    def add_words(self, words: list[str]):
        if self.disabled:
            return
        
        self.client.add_words(words)
