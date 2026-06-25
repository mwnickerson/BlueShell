from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class ExitArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = []

    async def parse_arguments(self):
        return


class ExitCommand(StandardTasking, CommandBase):
    cmd = "exit"
    help_cmd = "exit"
    description = "Terminate the callback."
    version = 1
    author = "root"
    argument_class = ExitArguments
    attributes = WINDOWS_COMMAND
    supported_ui_features = ["callback_table:exit"]
