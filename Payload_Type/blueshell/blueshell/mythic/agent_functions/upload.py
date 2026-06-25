import base64

from mythic_container.MythicCommandBase import *
from mythic_container.MythicGoRPC import *

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

    async def create_go_tasking(self, taskData):
        file_id = taskData.args.get_arg("file")
        result = await SendMythicRPCFileGetContent(
            MythicRPCFileGetContentMessage(AgentFileID=file_id)
        )
        if not result.Success:
            return PTTaskCreateTaskingMessageResponse(
                TaskID=taskData.Task.ID,
                Success=False,
                Error=result.Error,
            )
        remote_path = taskData.args.get_arg("remote_path")
        taskData.args.remove_arg("file")
        taskData.args.remove_arg("remote_path")
        taskData.args.add_arg("path", remote_path)
        taskData.args.add_arg(
            "data", base64.b64encode(result.Content).decode("ascii")
        )
        return PTTaskCreateTaskingMessageResponse(
            TaskID=taskData.Task.ID,
            Success=True,
            DisplayParams=remote_path,
        )
