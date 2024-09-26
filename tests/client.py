import string

from string_space import ProtocolError, StringSpaceClient

#######

def prexix_test(client):
    try:
        # create a list of all the lower case prefix letters
        prefixes = list(map(lambda x: x.lower(), string.ascii_letters))
        for prefix in prefixes:
            words = client.prefix_search(prefix=prefix)
            print(f"Prefix '{prefix}':")
            for word in words:
                print(f"  {word}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def substring_test(client):
    try:
        substring = "he"
        # Search by prefix
        found_strings = client.substring_search(substring)
        print(f"Strings with substring '{substring}':")
        for string_ref in found_strings:
            print(f"  {string_ref}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def insert_test(client):
    try:
        words_to_insert = ["hello", "helicopter", "help", "harmony", "hero", "rust"]
        result = client.insert(words_to_insert)
        print(result)
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def remove_test(client):
    try:
        words_to_remove = ["hello", "helicopter", "help", "harmony", "hero", "rust"]
        result = client.remove(words_to_remove)
        print(result)
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def get_all_strings_test(client):
    try:
        strings = client.get_all_strings()
        print(f"All strings:")
        for string_ref in strings:
            print(f"  {string_ref}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def empty_test(client):
    try:
        empty = client.empty()
        print(f"Empty: {empty}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def len_test(client):
    try:
        length = client.len()
        print(f"Length: {length}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def capacity_test(client):
    try:
        capacity = client.capacity()
        print(f"Capacity: {capacity}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def clear_space_test(client):
    try:
        client.clear_space()
        print(f"Cleared space")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def print_strings_test(client):
    try:
        client.print_strings()
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def similar_test(client):
    try:
        words = client.similar_search("testi", 0.6)
        print(f"Similar words:")
        for word in words:
            print(f"  {word}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

def data_file_test(client):
    try:
        data_file = client.data_file()
        print(f"Data file: {data_file}")
    except ProtocolError as e:
        print(f"ProtocolError: {e}")

import sys

def main():
    # read the first argument as the port number
    if len(sys.argv) < 2:
        print("Usage: python client.py <port>")
        sys.exit(1)
    port = int(sys.argv[1])
    client = StringSpaceClient('127.0.0.1', port)
    prexix_test(client)
    substring_test(client)
    similar_test(client)
    prefix = "testi"
    print("Prefix search:" + prefix)
    print("\n".join(client.prefix_search(prefix=prefix)))
    insert_test(client)
    data_file_test(client)
    # remove_test(client)
    # get_all_strings_test(client)
    # empty_test(client)
    # len_test(client)
    # capacity_test(client)
    # clear_space_test(client)
    # print_strings_test(client)

if __name__ == "__main__":
    main()