from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class UploadArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(name="file", type=ParameterType.File),
            CommandParameter(name="remote_path", type=ParameterType.String),
        ]

    async def parse_arguments(self):
        if not self.command_line.startswith("{"):
            raise ValueError("use the upload modal or supply JSON arguments")
        self.load_args_from_json_string(self.command_line)

    async def parse_dictionary(self, dictionary_arguments):
        self.load_args_from_dictionary(dictionary_arguments)


class UploadCommand(StandardTasking, CommandBase):
    cmd = "upload"
    help_cmd = "upload"
    description = "Upload a Mythic-hosted file to the target."
    version = 1
    author = "root"
    attackmapping = ["T1105"]
    argument_class = UploadArguments
    attributes = WINDOWS_COMMAND
    supported_ui_features = ["file_browser:upload"]
