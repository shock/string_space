# Model Autocomplete Implementation Master Plan

## Objective

**Primary Goal:** Implement a `prompt_toolkit` Completer that provides intelligent autocomplete suggestions for model names when users type the `/mod` command.

**Value Proposition:**
- **Enhanced User Experience**: Quick model selection without memorizing or copy/pasting exact model names
- **Provider-Aware Suggestions**: Intelligent completion across all configured providers
- **Flexible Input Support**: Support for long names, short names, and provider-prefixed formats
- **Seamless Integration**: Non-disruptive addition to existing chat interface
- **Performance Optimized**: Lightweight filtering with cached model data

## Core Guiding Principles

### Preservation of Existing Behavior

**Fundamental Rule:** The autocomplete feature must integrate seamlessly without disrupting existing functionality. This ensures:

- **No Interference**: Existing StringSpaceCompleter continues working normally
- **Command Integrity**: `/mod` command behavior remains unchanged
- **Performance Stability**: No degradation in typing responsiveness
- **UI Consistency**: Completion behavior follows existing patterns

### Backward Compatibility

- **Existing commands must work without changes** - `/mod` command behavior preserved
- **No breaking changes** to current chat interface functionality
- **Graceful degradation** when no providers are configured
- **Error resilience** - completer follows comprehensive error handling strategy (see Error Handling section)

## Current Architecture Analysis

### Existing Completer Architecture (ChatInterface.py:61-66)

The `ChatInterface` currently uses:
- `StringSpaceCompleter` (custom dependency connecting to external service on port 7878)
- `merge_completers([self.spell_check_completer])` to combine multiple completers
- Completer attached to `PromptSession` with `complete_while_typing=True`

**Key Observation:** The architecture already supports multiple merged completers, making integration straightforward, but we will need to modify it slightly to use a DelegatingCompleter for `/mod` detection to decide which completer to delegate to: the StringSpaceCompleter or the ModelCommandCompleter.

### Provider Manager Model Access (ProviderManager.py)

The `ProviderManager` class provides essential methods for model retrieval:
- **`valid_scoped_models()`**: Returns pre-formatted display strings - we WILL use this and modify it to support caching

### /mod Command Implementation (CommandHandler.py:85-89)

- Command: `/mod` with single model name argument
- Supports provider-prefixed (`openai/gpt-4o`), unprefixed (`gpt-4o`), and short name (`4o`) formats
- Uses `chat_interface.set_model()` for model switching

## Proposed Architecture

### File Organization and Component Overview

**New File Structure:**
- `modules/DelegatingCompleter.py` - Delegating completer class for A/B delegation based on command context
- `modules/ModelCommandCompleter.py` - Model command completer class for `/mod` command autocomplete
- `tests/test_ModelCommandCompleter.py` - Unit tests for model command completer
- `tests/test_DelegatingCompleter.py` - Unit tests for delegating completer
- Existing files requiring modification: `ChatInterface.py`, `tests/test_ChatInterface.py`

**Core Components:**

**DelegatingCompleter** - Routes completion requests:
- Inherits from Completer
- Delegates to either StringSpaceCompleter or ModelCommandCompleter based on command context
- Uses decision function that takes document as input and returns True for completer_a, False for completer_b
- Key methods: `__init__(completer_a, completer_b, decision_function)`, `get_completions(document, complete_event)`

### ModelCommandCompleter Class

**Responsibilities:**
- `/mod` command detection and context activation using regex matching
- Model name completion generation using filtered model list from ProviderManager.valid_scoped_models()
- Multiple completion format support (long names, short names, provider-prefixed)
- Case-insensitive filtering and ranking using jaro_winkler_metric: https://pypi.org/project/jaro-winkler/

**Key Methods:**
- `__init__(provider_manager)` - Initialize with ProviderManager instance
- `get_completions(document, complete_event)` - Core completion logic
- Internal helper methods for filtering and formatting

### Integration Strategy

**ChatInterface Integration:**
- Instantiate `ModelCommandCompleter` with existing ProviderManager
- Implement decision function to delegate to StringSpaceCompleter or ModelCommandCompleter based on document and matching regex
- Instantiate DelegatingCompleter with ModelCommandCompleter, existing self.merged_completer and decision function

**Completion Formats:**
- Provider-prefixed: `openai/gpt-4o`
- Short name only: `4o`
- Long name only (unprefixed): `gpt-4o`

## Implementation Steps

### Testing Strategy

**Integrated Testing Approach for Confidence at Each Step:**

1. **Pre-implementation**: Verify existing functionality baseline
2. **During each phase**: Unit tests for new methods + integration tests for completed components, if sensible
3. **Post-implementation**: Comprehensive regression testing and manual validation

**Testing Framework:**
- **Unit Tests**: Individual component testing with mocked dependencies
- **Mock Tests**: Controlled testing with mocked ProviderManager, document, and completion events
- **Integration Tests**: Testing component interactions and ChatInterface integration
- **Regression Tests**: Ensuring existing functionality remains unchanged
- **Manual QA**: Real-world testing with live chat interface

**Phase-by-Phase Testing Requirements:**
- **Phase 0**: Baseline verification tests for existing chat interface; Documentation of current `/mod` command behavior; Dependency installation verification
- **Phase 1**: Unit tests for `valid_scoped_models()` caching and invalidation
- **Phase 2**: Unit tests for `filter_completions()`, `get_model_substring()`, `get_completions()`; Mock tests with ProviderManager, document, and completion event
- **Phase 3**: Unit tests for DelegatingCompleter methods; Mock tests with completers and decision functions
- **Phase 4**: Integration verification tests for ChatInterface integration; Regression tests to ensure existing chat functionality unchanged
- **Phase 5**: KeyBindingsHandler interaction tests; Circular dependency validation; Integration validation with live chat interface
- **Phase 6**: Comprehensive unit tests for full completer functionality; End-to-end integration testing with complete chat interface; Manual QA with real-world scenarios
- **Phase 7**: Final regression tests running complete test suite; Documentation review for accuracy and completeness

Testing is integrated throughout each phase to ensure functionality confidence at every step.

### Phase 0: Foundational Analysis and Setup

1. **Analyze Current Completer Architecture**
   - Verify StringSpaceCompleter integration pattern
   - Confirm ProviderManager access patterns
   - Document existing completer behavior for regression testing

2. **Establish Baseline Tests**
   - Ensure existing chat interface tests pass
   - Document current `/mod` command behavior

3. **Dependency Management**
   - Add `jaro-winkler` package dependency using: `uv add jaro-winkler && uv sync`
   - **Note**: Package name is `jaro-winkler` but import uses `from jaro import jaro_winkler_metric`
   - Verify dependency installation and compatibility with existing packages

### Phase 1: Update ProviderManager

1. **Update ProviderManager**
   - Add `cached_valid_scoped_models` instance variable initialized to `None` in `__init__`
   - Update `valid_scoped_models()` to:
     - Check if `self.cached_valid_scoped_models` is not `None` and return cached results if available
     - If cache is empty, generate fresh results and store in `self.cached_valid_scoped_models`
     - Return the cached results
   - **Cache Invalidation Strategy**: Invalidate cache when `discover_models()` is run on one or more providers by setting `self.cached_valid_scoped_models = None`
   - **Implementation Details**:
     - Cache invalidation occurs at the start of `discover_models()` method before any model discovery begins
     - This ensures any subsequent calls to `valid_scoped_models()` will get fresh data reflecting the updated model lists
     - Cache remains valid until the next `discover_models()` operation


### Phase 2: Core ModelCommandCompleter Implementation

1. **Create ModelCommandCompleter.py**
   - Create new file `modules/ModelCommandCompleter.py`
   - Add imports: `from prompt_toolkit.completion import Completer, Completion`, `from typing import Iterable`, `import re`, `from jaro import jaro_winkler_metric`
   - Implement core completer class:
      ```python

      import re
      from prompt_toolkit.completion import Completer, Completion
      from jaro import jaro_winkler_metric


      class ModelCommandCompleter(Completer):
         def __init__(self, provider_manager, mod_command_pattern):
             self.provider_manager = provider_manager
             self.mod_command_pattern = mod_command_pattern

         def get_completions(self, document, complete_event):
             # Fetch model names from ProviderManager
             model_substring = self.get_model_substring(document)
             model_substring_len = len(model_substring)
             # remove all whitespace from model_substring
             model_substring = re.sub(r'\s', '', model_substring)
             if model_substring_len < 2 and not complete_event.completion_requested:
                 return

             # Error handling for ProviderManager calls
             try:
                 model_names = self.provider_manager.valid_scoped_models()
             except Exception as e:
                 # Log detailed error information to stderr for debugging
                 import sys
                 print(f"ModelCommandCompleter error: {e}", file=sys.stderr)
                 # Return empty completion list to maintain clean UX
                 return

             filtered_completions = self.filter_completions(model_names, model_substring)
             for completion in filtered_completions:
                 # Extract provider context from the formatted model string
                 provider_context = self.extract_provider_context(completion[0])
                 yield Completion(completion[0], start_position=-model_substring_len, display_meta=provider_context)

         def extract_provider_context(self, model_string):
             """Extract provider context from formatted model string for display_meta."""
             # Model string format: "provider/long_name (short_name)"
             if '/' in model_string:
                 provider = model_string.split('/')[0]
                 return f"{provider} model"
             return "Model"

         def filter_completions(self, model_names, model_substring):
             ranked_completions = substring_jaro_winkler_match(model_substring, model_names)
             return ranked_completions[:8]

         def get_model_substring(self, document):
            text = document.text_before_cursor
            matches = re.search(self.mod_command_pattern, text)
            if matches:
                return matches.group(1)
            else:
                return ''

      # Standalone function for substring matching using Jaro-Winkler similarity
      # This is a standalone function, not a class method, that can be used independently
      def substring_jaro_winkler_match(input_str, longer_strings):
         """
         Perform substring matching of an input string against a list of longer strings using Jaro-Winkler similarity.

         This is a standalone function that slides a window over each string in `longer_strings` with the same length as `input_str`,
         computes the Jaro-Winkler similarity between `input_str` and each substring, and records the highest similarity score
         for that string. It returns a list of tuples containing the original string and its best matching score,
         sorted in descending order of similarity.

         **Case-Insensitive Matching:** Both input strings are converted to lowercase before comparison to ensure
         case-insensitive matching for better user experience.

         Parameters:
         -----------
         input_str : str
            The input string to match as a substring.
         longer_strings : list of str
            A list of longer strings in which to search for the best substring match.

         Returns:
         --------
         list of tuples (str, float)
            A list of tuples where each tuple contains a string from `longer_strings` and its highest Jaro-Winkler similarity
            score with `input_str`. The list is sorted by similarity score in descending order.

         Example:
         --------
         >>> input_string = "martha"
         >>> longer_list = ["marhta", "marathon", "artha", "martian", "math"]
         >>> matches = substring_jaro_winkler_match(input_string, longer_list)
         >>> for string, score in matches:
         ...     print(f"{string}: {score:.4f}")
         marhta: 0.9611
         marathon: 0.8800
         artha: 0.8667
         martian: 0.8444
         math: 0.8000

         """
         input_len = len(input_str)
         results = []

         # Convert input string to lowercase for case-insensitive matching
         input_str_lower = input_str.lower()

         for long_str in longer_strings:
            max_score = 0.0
            # Convert longer string to lowercase for case-insensitive matching
            long_str_lower = long_str.lower()
            # Slide over the longer string with a window of input_str length
            for i in range(len(long_str_lower) - input_len + 1):
                  substring = long_str_lower[i:i+input_len]
                  score = jaro_winkler_metric(input_str_lower, substring)
                  if score > max_score:
                     max_score = score
            results.append((long_str, max_score))

         # Sort by score descending
         results.sort(key=lambda x: x[1], reverse=True)
         return results
      ```



### Phase 3: Delegation Completer

1. **Create DelegatingCompleter**
   - Create new file `modules/DelegatingCompleter.py`
   - Add imports: `from prompt_toolkit.completion import Completer, Completion`, `from typing import Iterable`
   - Implement DelegatingCompleter class:
      ```python
      class DelegatingCompleter(Completer):
         def __init__(self, completer_a, completer_b, decision_function):
            self.completer_a = completer_a
            self.completer_b = completer_b
            self.decision_function = decision_function

         def get_completions(self, document, complete_event):
            if self.decision_function(document):
                  yield from self.completer_a.get_completions(document, complete_event)
            else:
                  yield from self.completer_b.get_completions(document, complete_event)
      ```


### Phase 4: ChatInterface Integration

1. **Complete ChatInterface Integration**
   - **Import New Modules**: Add imports for `ModelCommandCompleter` and `DelegatingCompleter` in `ChatInterface.py`
   - **Define Decision Function**: Implement `is_mod_command()` function at the end of `ChatInterface.py` using the `MOD_COMMAND_PATTERN` regex
      ```python

      MOD_COMMAND_PATTERN = re.compile(r'^\s*\/mod[^\s]*\s+([^\s]*)')

      def is_mod_command(document) -> bool:
         text = document.text_before_cursor
         match = re.match(MOD_COMMAND_PATTERN, text)
         if match:
            return True
         return False
      ```
   - **Instantiate Components**: In `ChatInterface.__init__()` around lines 61-66:
     - Instantiate `ModelCommandCompleter` with ProviderManager instance and `MOD_COMMAND_PATTERN`
     - Instantiate `DelegatingCompleter` with `ModelCommandCompleter`, existing `self.merged_completer`, and `is_mod_command` decision function
     - Assign `DelegatingCompleter` instance to `self.top_level_completer`
   - **Update PromptSession**: Replace `self.merged_completer` with `self.top_level_completer` in `PromptSession` initialization

2. **Update Dependencies and Imports**
   - Add import for `re`
   - Add import for ModelCommandCompleter and DelegatingCompleter in ChatInterface
   - Verify no circular dependencies
   - Maintain existing import structure


### Phase 5: Integration Validation and Testing

1. **Validate KeyBindingsHandler Interactions**
   - **Review KeyBindingsHandler**: Examine `KeyBindingsHandler` class in `modules/KeyBindingsHandler.py` for any completer-specific behavior
   - **Test Tab Completion**: Verify that Tab key behavior works correctly with the new DelegatingCompleter
   - **Check Custom Key Bindings**: Ensure no custom key bindings interfere with completion behavior
   - **Validate Auto-completion**: Test that `complete_while_typing=True` setting still works as expected

2. **Update Existing Chat Interface Tests**
   - **Review Current Tests**: Examine `tests/test_ChatInterface.py` for existing completer-related tests
   - **Add Integration Tests**: Create new test methods to verify:
     - DelegatingCompleter properly routes between StringSpaceCompleter and ModelCommandCompleter
     - `/mod` command context detection works correctly
     - Non-`/mod` commands continue using StringSpaceCompleter
   - **Update Mock Tests**: Modify existing mock tests to account for the new completer architecture
   - **Test Edge Cases**: Add tests for empty input, partial `/mod` commands, and error scenarios

3. **Ensure No Circular Dependencies**
   - **Import Analysis**: Verify import structure doesn't create circular dependencies:
     - `ChatInterface` imports `ModelCommandCompleter` and `DelegatingCompleter`
     - `ModelCommandCompleter` imports `ProviderManager`
     - `ProviderManager` should NOT import any completer classes
   - **Test Import Chain**: Run import tests to ensure all modules can be imported without circular dependency errors
   - **Verify Runtime**: Test that the application starts without circular dependency issues

4. **Integration Validation**
   - **Manual Testing**: Perform manual testing with live chat interface to verify:
     - `/mod` command triggers model autocomplete
     - Other commands continue using spell check completer
     - Tab completion works correctly
     - No performance degradation
   - **Error Scenario Testing**: Test error handling when ProviderManager throws exceptions
   - **Provider Configuration Testing**: Test with multiple providers configured and with no providers configured

### Phase 6: Comprehensive Testing and Validation

1. **Create Unit Test Suite**
   - Create `tests/test_ModelCommandCompleter.py`
   - Test all completion scenarios and edge cases including:
     - **Empty model lists**: Test behavior when ProviderManager returns empty list
     - **Special characters in model names**: Test completion with model names containing hyphens, underscores, dots, and other special characters
     - **Very long model names**: Test completion with extremely long model names (100+ characters)
     - **Unicode characters**: Test completion with model names containing non-ASCII characters
     - **Mixed case model names**: Test case-insensitive matching with mixed case model names
     - **Partial matches**: Test completion with various partial input strings
     - **Exact matches**: Test completion when input exactly matches model names
     - **No matches**: Test behavior when no models match the input
     - **Whitespace handling**: Test completion with leading/trailing whitespace in input
     - **Provider prefix variations**: Test completion with different provider prefix formats
     - **Error scenarios**: Test error handling when ProviderManager raises exceptions
     - **Performance boundaries**: Test completion with large model lists (100+ models)
   - Mock ProviderManager for controlled testing
   - Attempt >90% test coverage

2. **Extend Integration Tests**
   - Update `tests/test_ChatInterface.py`
   - Test completer integration with mock Document objects
   - Verify no interference with existing features

3. **Manual Testing and Validation**
   - Test with live chat interface
   - Verify completion behavior with various input scenarios
   - Test with multiple providers configured
   - Validate error handling and graceful degradation


### Phase 7: Final Polish and Documentation

1. **Code Quality and Optimization**
   - Code review and optimization

2. **Documentation Updates**
   - Update module documentation
   - Update in-app help to briefly explain new feature


## Key Design Decisions

### Completion Strategy

**Matching Strategy:**
- Use Jaro-Winkler similarity for string matching
- Allows maximum flexibility for user input
- Maintains consistency with existing `/mod` command behavior

**Display Strategy:**
- Use `display_meta` for provider context (e.g., "openai model", "anthropic model")
- Extract provider information from formatted model strings using `extract_provider_context()` method
- Keep completion text clean for insertion
- Sort by string length for quick selection

### Performance Optimization

**Caching Strategy:**
- Cache model list only when not already cached
- Force cache refresh by setting `self.cached_valid_scoped_models = None` in ProviderManager
- **Cache Invalidation**: See Phase 1 implementation for detailed cache invalidation strategy and triggers

**Filtering Efficiency:**
- Case-insensitive matching for better UX
- Partial matching for flexible input
- Early return when not in `/mod` context

### Error Handling

**Comprehensive Error Handling Strategy:**

**ProviderManager Exceptions:**
- Catch all ProviderManager exceptions gracefully
- Return empty completion list when exceptions occur
- Print detailed error information to stderr for debugging
- No user-facing error messages to maintain clean UX

**Graceful Degradation:**
- Handle empty provider configurations gracefully
- Return empty completion list when no providers are configured
- No completion interference with other commands
- Seamless fallback to manual input when autocomplete fails

**User Experience:**
- Clear completion suggestions when available
- No disruptive behavior when errors occur
- Maintains existing chat functionality during failures

## Technical Implementation Details

### Prompt Toolkit Integration

**Required Imports:**
```python
from prompt_toolkit.completion import Completer, Completion
from prompt_toolkit.document import Document
from jaro import jaro_winkler_metric
```

**Completion Object Structure:**
- `text`: The completion string to insert (e.g., "openai/gpt-4o (4o)")
- `start_position`: Characters to replace (length of partial word)
- `display`: Optional rich display text
- `display_meta`: Provider context information (e.g., "openai model", "anthropic model")

### ProviderManager Integration

**Access Pattern:**
```python
# In ChatInterface.__init__
# ProviderManager instance is already available at self.config.config.providers
# This is the exact access pattern to use when instantiating ModelCommandCompleter
self.model_completer = ModelCommandCompleter(self.config.config.providers)
```

**Implementation Details:**
- The ProviderManager instance is accessible via `self.config.config.providers` in ChatInterface
- This access pattern provides direct access to the existing ProviderManager instance
- No additional ProviderManager instantiation is required - use the existing instance
- This ensures consistency with the existing configuration architecture

### Regex Pattern for Command Detection

**/mod Command Detection:**
```python
MOD_COMMAND_PATTERN = re.compile(r'^\s*\/mod[^\s]*\s+([^\s]*)')
```

**Context Activation:**
- Only activate when pattern matches
- Extract partial model name after `/mod ` prefix
- Return without yielding when not in context

## Benefits of New Architecture

1. **Enhanced User Experience**
   - Quick model selection without memorization
   - Intelligent suggestions across all providers
   - Flexible input format support

2. **Seamless Integration**
   - Non-disruptive addition to existing interface
   - No conflicts with existing completers
   - Maintains existing command behavior

3. **Performance Optimized**
   - Cached model names
   - Minimal impact on typing responsiveness

4. **Extensible Design**
   - Easy to add new completion formats
   - Support for future provider enhancements
   - Clean separation of concerns

## Risk Assessment

**Low Risk Areas:**
- Completer integration pattern well-established
- ProviderManager interface stable
- Existing tests provide good coverage

**Medium Risk Areas:**
- Edge case handling in filtering logic

**High Risk Areas:**
- Complex regex pattern for command detection needs testing

**Mitigation Strategies:**
- Comprehensive testing at each phase

## Success Criteria

- [ ] Completer activates only for `/mod` commands
- [ ] Suggestions include long names, short names, and provider names, as returned by ProviderManager.valid_scoped_models()
- [ ] Tab completion works correctly
- [ ] No interference with existing StringSpaceCompleter
- [ ] All existing tests pass
- [ ] Manual testing confirms expected behavior
- [ ] Case-insensitive matching works correctly
- [ ] Comprehensive error handling strategy implemented (see Error Handling section)
- [ ] No performance degradation in typing responsiveness - manual testing required

## Migration Considerations

- **No breaking changes** to existing functionality
- **Gradual enhancement** - completer is additive only
- **Backward compatibility** - existing `/mod` usage patterns preserved
- **Error resilience** - completer fails gracefully without disrupting chat

## Technical Implementation Notes

*Note: Comprehensive error handling is detailed in the Error Handling section above.*

---

## PLAN REVIEW RESULTS

### Redundancies Found:

### Inconsistencies Found:

### Missing Critical Details:
