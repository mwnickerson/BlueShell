from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class PwdArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = []

    async def parse_arguments(self):
        return


class PwdCommand(StandardTasking, CommandBase):
    cmd = "pwd"
    help_cmd = "pwd"
    description = "Print the callback working directory."
    version = 1
    author = "root"
    argument_class = PwdArguments
    attributes = WINDOWS_COMMAND
