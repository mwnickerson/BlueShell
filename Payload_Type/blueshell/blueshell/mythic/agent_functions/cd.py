from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class CdArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [CommandParameter(name="path", type=ParameterType.String)]

    async def parse_arguments(self):
        if not self.command_line.strip():
            raise ValueError("a path is required")
        self.add_arg("path", self.command_line.strip())


class CdCommand(StandardTasking, CommandBase):
    cmd = "cd"
    help_cmd = "cd <path>"
    description = "Change the callback working directory."
    version = 1
    author = "root"
    argument_class = CdArguments
    attributes = WINDOWS_COMMAND
