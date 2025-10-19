#!/usr/bin/env python3

import signal
from prompt_toolkit import PromptSession
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit.styles import Style
from prompt_toolkit.formatted_text import HTML
from string_space_completer import StringSpaceCompleter
from prompt_toolkit.completion import merge_completers
from prompt_toolkit.history import InMemoryHistory


class SigTermException(Exception):
    pass

class ChatInterface:
    """Class to provide a chat interface."""

    def __init__(self,):
        self.spell_check_completer = StringSpaceCompleter(host='127.0.0.1', port=7878)
        self.merged_completer = merge_completers([self.spell_check_completer])

        self.session = PromptSession(
            history=InMemoryHistory(),
            completer=self.merged_completer,
            complete_while_typing=True,
            auto_suggest=None,
        )
        self.session.app.ttimeoutlen = 0.001  # Set to 1 millisecond
        # Register the signal handler for SIGTERM
        signal.signal(signal.SIGTERM, self.signal_handler)

    def signal_handler(self, _sig, _frame):  # parameters are required by signal handler signature but not used
        raise SigTermException()

    def run(self):
        try:
            while True:
                try:
                    self.spell_check_completer.stop() # Shouldn't be necessary, but it is
                    prompt_symbol = f'>'
                    user_input = self.session.prompt(
                        HTML(f'<style fg="white">{prompt_symbol}</style> '),
                        style=Style.from_dict({'': 'white'}),
                        multiline=True
                    )
                    if user_input is None or user_input.strip() == '':
                        continue
                    else:
                        self.spell_check_completer.add_words_from_text(user_input)

                except EOFError:
                    break
                except KeyboardInterrupt:
                    pass
        except KeyboardInterrupt:
            pass
        except SigTermException:
            pass
        self.spell_check_completer.stop()


if __name__ == '__main__':
    chat_interface = ChatInterface()
    chat_interface.run()