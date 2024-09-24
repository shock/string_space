# Author: Bill Doughty
# Version: 1.0

import socket
import string
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
        try:
            self.connect()
        except ConnectionRefusedError as e:
            pass

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)  # Create a new socket
        try:
            self.sock.connect((self.host, self.port))
            self.connected = True
        except ConnectionRefusedError as e:
            print(f"WARNING: StringSpaceServer unreachable.  Is it running on {self.host}:{self.port}?")
            print(f"Will retry...")
            self.sock.close()
            self.connected = False
            raise e

    def disconnect(self):
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
            return []

    def substring_search(self, substring: str) -> list[str]:
        try:
            request_elements = ["substring", substring]
            response = self.request(request_elements)
            return response.split('\n')
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return []

    def similar_search(self, word: str, threshold: int) -> list[str]:
        try:
            request_elements = ["similar", word, str(threshold)]
            response = self.request(request_elements)
            return response.split('\n')
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return []

    def insert(self, strings: list[str]):
        try:
            request_elements = ["insert", "\n".join(strings)]
            response = self.request(request_elements)
            return response
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return []

    def add_words(self, words):
        try:
            request_elements = ["add_words", "\n".join(words)]
            response = self.request(request_elements)
            return response
        except ProtocolError as e:
            if self.debug:
                print(f"Error: {e}")
            return "ERROR"