from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class LsArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(
                name="path", type=ParameterType.String, default_value="."
            )
        ]

    async def parse_arguments(self):
        self.add_arg("path", self.command_line.strip() or ".")


class LsCommand(StandardTasking, CommandBase):
    cmd = "ls"
    help_cmd = "ls [path]"
    description = "List files and directories."
    version = 1
    author = "root"
    argument_class = LsArguments
    attributes = WINDOWS_COMMAND
    supported_ui_features = ["file_browser:list"]
