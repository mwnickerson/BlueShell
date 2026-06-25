from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class DownloadArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [CommandParameter(name="path", type=ParameterType.String)]

    async def parse_arguments(self):
        if not self.command_line.strip():
            raise ValueError("a remote path is required")
        self.add_arg("path", self.command_line.strip())


class DownloadCommand(StandardTasking, CommandBase):
    cmd = "download"
    help_cmd = "download <remote path>"
    description = "Download a file from the target."
    version = 1
    author = "root"
    attackmapping = ["T1105"]
    argument_class = DownloadArguments
    attributes = WINDOWS_COMMAND
    supported_ui_features = ["file_browser:download"]
