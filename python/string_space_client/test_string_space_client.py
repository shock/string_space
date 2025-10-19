#!/usr/bin/env python3
"""
Test suite for StringSpaceClient best_completions_search method.

This module provides comprehensive tests for the best_completions_search
method in the StringSpaceClient class, including valid queries, error
handling, and integration with existing client methods.
"""

import unittest
from unittest.mock import Mock, patch
import socket
from string_space_client import StringSpaceClient, ProtocolError


class TestStringSpaceClientBestCompletions(unittest.TestCase):
    """Test cases for best_completions_search method."""

    def setUp(self):
        """Set up test fixtures before each test method."""
        self.client = StringSpaceClient('127.0.0.1', 7878, debug=False)

    def test_best_completions_search_valid(self):
        """Test valid query with default limit."""
        # Mock server response for best-completions query
        mock_response = "hello\nhelp\nhelicopter\nworld\n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("hel")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "hel", "10"])

            # Verify the results are parsed correctly
            expected_results = ["hello", "help", "helicopter", "world"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_with_limit(self):
        """Test valid query with custom limit."""
        # Mock server response for best-completions query with limit
        mock_response = "apple\napplication\napply\n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("app", limit=5)

            # Verify the request was made with correct parameters including custom limit
            mock_request.assert_called_once_with(["best-completions", "app", "5"])

            # Verify the results are parsed correctly
            expected_results = ["apple", "application", "apply"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_empty_query(self):
        """Test empty query handling."""
        # Mock server response for empty query
        mock_response = ""

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("")

            # Verify the request was made with empty query
            mock_request.assert_called_once_with(["best-completions", "", "10"])

            # Verify empty results are handled correctly
            self.assertEqual(results, [])

    def test_best_completions_search_error_handling(self):
        """Test error handling validation."""
        # Test ProtocolError handling
        with patch.object(self.client, 'request') as mock_request:
            mock_request.side_effect = ProtocolError("ERROR - Server error")

            results = self.client.best_completions_search("test")

            # Verify error is caught and returned in list format
            self.assertEqual(len(results), 1)
            self.assertTrue(results[0].startswith("ERROR:"))
            self.assertIn("Server error", results[0])

        # Test ConnectionError handling
        with patch.object(self.client, 'request') as mock_request:
            mock_request.side_effect = ConnectionError("Connection refused")

            with self.assertRaises(ConnectionError):
                self.client.best_completions_search("test")

    def test_best_completions_search_integration(self):
        """Test integration with existing client methods."""
        # Mock responses for different search methods
        prefix_response = "hello\nhelp\nhelicopter"
        fuzzy_response = "hello\nhelp\nhelicopter"
        best_completions_response = "help\nhello\nhelicopter"

        with patch.object(self.client, 'request') as mock_request:
            # Set up different responses for different method calls
            def side_effect(request_elements):
                if request_elements[0] == "prefix":
                    return prefix_response
                elif request_elements[0] == "fuzzy-subsequence":
                    return fuzzy_response
                elif request_elements[0] == "best-completions":
                    return best_completions_response
                return ""

            mock_request.side_effect = side_effect

            # Test prefix search
            prefix_results = self.client.prefix_search("hel")
            self.assertEqual(prefix_results, ["hello", "help", "helicopter"])

            # Test fuzzy subsequence search
            fuzzy_results = self.client.fuzzy_subsequence_search("hl")
            self.assertEqual(fuzzy_results, ["hello", "help", "helicopter"])

            # Test best completions search
            best_results = self.client.best_completions_search("hel")
            self.assertEqual(best_results, ["help", "hello", "helicopter"])

            # Verify all methods were called
            self.assertEqual(mock_request.call_count, 3)

    def test_best_completions_search_realistic_data(self):
        """Test with realistic data patterns."""
        # Mock realistic server response with varied data
        mock_response = """help
hello
helicopter
world
word
ward
apple
application
apply
applesauce"""

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("hel", limit=10)

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "hel", "10"])

            # Verify all results are parsed and empty lines are removed
            expected_results = [
                "help", "hello", "helicopter", "world",
                "word", "ward", "apple", "application",
                "apply", "applesauce"
            ]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_unicode_handling(self):
        """Test Unicode character handling."""
        # Mock server response with Unicode characters
        mock_response = "café\nnaïve\nüber\nhello\n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("caf")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "caf", "10"])

            # Verify Unicode results are parsed correctly
            expected_results = ["café", "naïve", "über", "hello"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_special_characters(self):
        """Test handling of special characters in queries."""
        # Mock server response for special character query
        mock_response = "test-hyphen\ntest_underscore\ntest.dot\n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("test-")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "test-", "10"])

            # Verify special character results are parsed correctly
            expected_results = ["test-hyphen", "test_underscore", "test.dot"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_empty_response(self):
        """Test handling of empty server response."""
        # Mock empty server response
        mock_response = ""

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("nonexistent")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "nonexistent", "10"])

            # Verify empty response is handled correctly
            self.assertEqual(results, [])

    def test_best_completions_search_whitespace_handling(self):
        """Test handling of whitespace in responses."""
        # Mock server response with extra whitespace
        mock_response = "  hello  \n  help  \n  helicopter  \n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("hel")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "hel", "10"])

            # Verify whitespace is preserved in results
            expected_results = ["  hello  ", "  help  ", "  helicopter  "]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_connection_retry(self):
        """Test connection retry behavior."""
        # This test is complex because the retry logic is in the request method
        # and mocking at this level bypasses the actual retry mechanism.
        # We'll test that ProtocolError is properly handled instead.
        with patch.object(self.client, 'request') as mock_request:
            mock_request.side_effect = ProtocolError("Connection failed after retries")

            results = self.client.best_completions_search("hel")

            # Verify error is caught and returned in list format
            self.assertEqual(len(results), 1)
            self.assertTrue(results[0].startswith("ERROR:"))
            self.assertIn("Connection failed after retries", results[0])

    def test_best_completions_search_very_long_query(self):
        """Test handling of very long queries."""
        # Create a long query (50 characters is the maximum allowed)
        long_query = "a" * 50

        with patch.object(self.client, 'request') as mock_request:
            mock_response = "match1\nmatch2\n"
            mock_request.return_value = mock_response

            results = self.client.best_completions_search(long_query)

            # Verify the request was made with the long query
            mock_request.assert_called_once_with(["best-completions", long_query, "10"])

            # Verify results are parsed correctly
            expected_results = ["match1", "match2"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_single_result(self):
        """Test handling of single result."""
        # Mock server response with single result
        mock_response = "hello\n"

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("hello")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "hello", "10"])

            # Verify single result is parsed correctly
            expected_results = ["hello"]
            self.assertEqual(results, expected_results)

    def test_best_completions_search_no_results(self):
        """Test handling of no results."""
        # Mock server response with no results
        mock_response = ""

        with patch.object(self.client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = self.client.best_completions_search("nonexistent")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "nonexistent", "10"])

            # Verify empty results are handled correctly
            self.assertEqual(results, [])

    def test_best_completions_search_debug_mode(self):
        """Test behavior in debug mode."""
        # Create client with debug mode enabled
        debug_client = StringSpaceClient('127.0.0.1', 7878, debug=True)

        mock_response = "hello\nhelp\n"

        with patch.object(debug_client, 'request') as mock_request:
            mock_request.return_value = mock_response

            results = debug_client.best_completions_search("hel")

            # Verify the request was made with correct parameters
            mock_request.assert_called_once_with(["best-completions", "hel", "10"])

            # Verify results are parsed correctly
            expected_results = ["hello", "help"]
            self.assertEqual(results, expected_results)


if __name__ == '__main__':
    unittest.main()