from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class ShellArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [CommandParameter(name="command", type=ParameterType.String)]

    async def parse_arguments(self):
        if not self.command_line.strip():
            raise ValueError("a command is required")
        self.add_arg("command", self.command_line)


class ShellCommand(StandardTasking, CommandBase):
    cmd = "shell"
    needs_admin = False
    help_cmd = "shell <command>"
    description = "Execute a command through the system command interpreter."
    version = 1
    author = "root"
    attackmapping = ["T1059.003"]
    argument_class = ShellArguments
    attributes = WINDOWS_COMMAND
