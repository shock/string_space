# Author: Bill Doughty
# Version: 0.3.0
# Date: 2025-10-07

import socket
import re
import time

RS_BYTE = 0x1E
RS_BYTE_STR = "\x1E"

class ProtocolError(Exception):
    pass

class StringSpaceClient:
    def __init__(self, host, port, debug=False):
        self.host = host
        self.port = port
        self.debug = debug
        self.connected = False

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)  # Create a new socket
        try:
            self.sock.connect((self.host, self.port))
            self.connected = True
        except ConnectionRefusedError as e:
            self.sock.close()
            self.connected = False
            raise ProtocolError(f"StringSpaceServer unreachable.  Is it running on {self.host}:{self.port}?")


    def disconnect(self):
        if self.connected:
            self.sock.close()
            self.connected = False

    def __del__(self):
        self.disconnect()

    def create_request(self, string: str):
        req = bytearray()
        req.extend(string.encode('utf-8'))
        req.extend(b'\x04')  # Append the EOT byte (ASCII EOT character)
        return req

    def receive_response(self):
        try:
            data = b''
            while True:
                chunk = self.sock.recv(4096)
                if not chunk:
                    raise ConnectionError("Connection closed by the server")
                data += chunk
                if b'\x04' in chunk:
                    break
            result = data.rstrip(b'\x04').decode('utf-8')
            # check the first 5 characters of the response to see if it's an error
            if result[:5] == "ERROR":
                raise ProtocolError(result)
            return result
        except Exception as e:
            if self.debug:
                print(f"Error: {e}")
            raise e

    def request(self, request_elements: list[str]) -> str:
        request = self.create_request(RS_BYTE_STR.join(request_elements))
        retries = 0
        max_retries = 2
        if not self.connected:
            try:
                # pass
                self.connect()
            except ConnectionRefusedError as e:
                raise ProtocolError(f"StringSpaceServer unreachable.  Is it running on {self.host}:{self.port}?")
        while True:
            try:
                self.sock.sendall(request)
                if self.debug:
                    print(f"Request sent: {request}")

                response = self.receive_response()
                if self.debug:
                    print(f"Response:\n{response}")
                self.disconnect()
                return response
            except ConnectionError as e:
                if self.debug:
                    print(f"Error: {e}")
                if retries < max_retries:
                    # sleep for 2^retries seconds
                    time.sleep(2**retries)
                    retries += 1
                    self.connect()
                    continue
                else:
                    raise e

    def prefix_search(self, prefix: str) -> list[str]:
        try:
            request_elements = ["prefix", prefix]
            response = self.request(request_elements)
            return response.split('\n')
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return [f"ERROR: {e}"]

    def substring_search(self, substring: str) -> list[str]:
        try:
            request_elements = ["substring", substring]
            response = self.request(request_elements)
            return response.split('\n')
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return [f"ERROR: {e}"]

    def similar_search(self, word: str, threshold: float) -> list[str]:
        try:
            request_elements = ["similar", word, str(threshold)]
            response = self.request(request_elements)
            return response.split('\n')
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return [f"ERROR: {e}"]

    def fuzzy_subsequence_search(self, query: str) -> list[str]:
        """
        Perform fuzzy-subsequence search for strings where query characters appear in order.

        Args:
            query: The subsequence pattern to search for

        Returns:
            list[str]: List of matching strings, or error message in list format
        """
        try:
            request_elements = ["fuzzy-subsequence", query]
            response = self.request(request_elements)
            # Remove empty strings from the result (consistent with other search methods)
            return [line for line in response.split('\n') if line]
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return [f"ERROR: {e}"]

    def data_file(self) -> str:
        try:
            request_elements = ["data-file"]
            response = self.request(request_elements)
            return response
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return f"ERROR: {e}"

    def insert(self, strings: list[str]):
        try:
            request_elements = ["insert", "\n".join(strings)]
            response = self.request(request_elements)
            return response
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return f"ERROR: {e}"

    def add_words(self, words):
        try:
            request_elements = ["insert", "\n".join(words)]
            response = self.request(request_elements)
            return response
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return f"ERROR: {e}"

    def parse_text(self, text: str) -> list[str]:
        # split the text into words by regex /s+/
        words = re.split(r'[^\w_\-s]+', text)
        # get unique words
        words = list(set(words))
        # filter out words that are too short or too long
        words = [word for word in words if len(word) >= 3]
        return words

    def add_words_from_text(self, text: str):
        words = self.parse_text(text)
        response = self.add_words(words)
        return response
