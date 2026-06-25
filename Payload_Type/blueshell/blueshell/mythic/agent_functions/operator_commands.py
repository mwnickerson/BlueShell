"""Operator-focused command definitions shared by both BlueShell stages."""

import base64

from mythic_container.MythicCommandBase import *
from mythic_container.MythicGoRPC import *

from .common import StandardTasking, WINDOWS_COMMAND


class OptionalStringArguments(TaskArguments):
    parameter_name = "arguments"

    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(name=self.parameter_name, type=ParameterType.String)
        ]

    async def parse_arguments(self):
        self.add_arg(self.parameter_name, self.command_line.strip())


class RequiredStringArguments(OptionalStringArguments):
    async def parse_arguments(self):
        if not self.command_line.strip():
            raise ValueError(f"{self.parameter_name} is required")
        await super().parse_arguments()


class FingerprintCommand(StandardTasking, CommandBase):
    cmd = "fingerprint"
    help_cmd = "fingerprint"
    description = "Return a stable host fingerprint and callback context."
    version = 1
    author = "root"
    argument_class = OptionalStringArguments
    attributes = WINDOWS_COMMAND


class PsCommand(StandardTasking, CommandBase):
    cmd = "ps"
    help_cmd = "ps"
    description = "List processes."
    version = 1
    author = "root"
    attackmapping = ["T1057"]
    argument_class = OptionalStringArguments
    attributes = WINDOWS_COMMAND
    supported_ui_features = ["process_browser:list"]


class KillArguments(RequiredStringArguments):
    parameter_name = "pid"


class KillCommand(StandardTasking, CommandBase):
    cmd = "kill"
    help_cmd = "kill <pid>"
    description = "Terminate a process."
    version = 1
    author = "root"
    attackmapping = ["T1489"]
    argument_class = KillArguments
    attributes = WINDOWS_COMMAND


class CoffArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(name="object", type=ParameterType.File),
            CommandParameter(
                name="entrypoint", type=ParameterType.String, default_value="go"
            ),
            CommandParameter(
                name="arguments", type=ParameterType.String, default_value=""
            ),
        ]

    async def parse_arguments(self):
        if not self.command_line.startswith("{"):
            raise ValueError("use the COFF task modal or JSON arguments")
        self.load_args_from_json_string(self.command_line)

    async def parse_dictionary(self, dictionary_arguments):
        self.load_args_from_dictionary(dictionary_arguments)


class CoffCommand(StandardTasking, CommandBase):
    cmd = "coff"
    help_cmd = "coff"
    description = "Execute a COFF object in the current process."
    version = 1
    author = "root"
    attackmapping = ["T1106"]
    argument_class = CoffArguments
    attributes = WINDOWS_COMMAND

    async def opsec_pre(self, taskData):
        return PTTTaskOPSECPreTaskMessageResponse(
            TaskID=taskData.Task.ID,
            Success=True,
            OpsecPreBlocked=True,
            OpsecPreBypassRole="operator",
            OpsecPreMessage="This task executes native object code in-process.",
        )

    async def create_go_tasking(self, taskData):
        file_id = taskData.args.get_arg("object")
        result = await SendMythicRPCFileGetContent(
            MythicRPCFileGetContentMessage(AgentFileID=file_id)
        )
        if not result.Success:
            return PTTaskCreateTaskingMessageResponse(
                TaskID=taskData.Task.ID,
                Success=False,
                Error=result.Error,
            )

        entrypoint = taskData.args.get_arg("entrypoint")
        arguments = taskData.args.get_arg("arguments") or ""
        taskData.args.remove_arg("object")
        taskData.args.remove_arg("arguments")
        taskData.args.add_arg(
            "object", base64.b64encode(result.Content).decode("ascii")
        )
        taskData.args.add_arg(
            "arguments", base64.b64encode(arguments.encode()).decode("ascii")
        )
        return PTTaskCreateTaskingMessageResponse(
            TaskID=taskData.Task.ID,
            Success=True,
            DisplayParams=f"{entrypoint} ({file_id})",
        )


class Stage1Arguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [CommandParameter(name="payload", type=ParameterType.File)]

    async def parse_arguments(self):
        if not self.command_line.startswith("{"):
            raise ValueError("use the stage1 task modal or JSON arguments")
        self.load_args_from_json_string(self.command_line)

    async def parse_dictionary(self, dictionary_arguments):
        self.load_args_from_dictionary(dictionary_arguments)


class Stage1Command(StandardTasking, CommandBase):
    cmd = "stage1"
    help_cmd = "stage1"
    description = "Retrieve and launch a BlueShell stage1 payload."
    version = 1
    author = "root"
    attackmapping = ["T1105", "T1055"]
    argument_class = Stage1Arguments
    attributes = WINDOWS_COMMAND


class ProxyArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(
                name="action",
                type=ParameterType.ChooseOne,
                choices=["start", "stop"],
                default_value="start",
            ),
            CommandParameter(name="port", type=ParameterType.Number),
        ]

    async def parse_arguments(self):
        if self.command_line.startswith("{"):
            self.load_args_from_json_string(self.command_line)
            return
        values = self.command_line.split()
        if len(values) != 2:
            raise ValueError("expected <start|stop> <port>")
        self.add_arg("action", values[0])
        self.add_arg("port", int(values[1]))


class SocksCommand(StandardTasking, CommandBase):
    cmd = "socks"
    help_cmd = "socks <start|stop> <server port>"
    description = "Start or stop a Mythic SOCKS proxy."
    version = 1
    author = "root"
    argument_class = ProxyArguments
    attributes = WINDOWS_COMMAND

    async def create_go_tasking(self, taskData):
        action = taskData.args.get_arg("action")
        port = taskData.args.get_arg("port")
        message = (
            MythicRPCProxyStartMessage(
                TaskID=taskData.Task.ID, LocalPort=port, PortType="socks"
            )
            if action == "start"
            else MythicRPCProxyStopMessage(
                TaskID=taskData.Task.ID, LocalPort=port, PortType="socks"
            )
        )
        result = (
            await SendMythicRPCProxyStartCommand(message)
            if action == "start"
            else await SendMythicRPCProxyStopCommand(message)
        )
        return PTTaskCreateTaskingMessageResponse(
            TaskID=taskData.Task.ID,
            Success=result.Success,
            Error=result.Error,
            DisplayParams=f"{action} {port}",
        )


class RpfwdArguments(ProxyArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args.extend(
            [
                CommandParameter(
                    name="remote_ip", type=ParameterType.String, default_value=""
                ),
                CommandParameter(
                    name="remote_port", type=ParameterType.Number, default_value=0
                ),
            ]
        )

    async def parse_arguments(self):
        if not self.command_line.startswith("{"):
            raise ValueError("use the rpfwd task modal or JSON arguments")
        self.load_args_from_json_string(self.command_line)


class RpfwdCommand(StandardTasking, CommandBase):
    cmd = "rpfwd"
    help_cmd = "rpfwd"
    description = "Start or stop a reverse port forward."
    version = 1
    author = "root"
    argument_class = RpfwdArguments
    attributes = WINDOWS_COMMAND

    async def create_go_tasking(self, taskData):
        action = taskData.args.get_arg("action")
        port = taskData.args.get_arg("port")
        if action == "start":
            result = await SendMythicRPCProxyStartCommand(
                MythicRPCProxyStartMessage(
                    TaskID=taskData.Task.ID,
                    LocalPort=port,
                    RemoteIP=taskData.args.get_arg("remote_ip"),
                    RemotePort=taskData.args.get_arg("remote_port"),
                    PortType="rpfwd",
                )
            )
        else:
            result = await SendMythicRPCProxyStopCommand(
                MythicRPCProxyStopMessage(
                    TaskID=taskData.Task.ID,
                    LocalPort=port,
                    PortType="rpfwd",
                )
            )
        return PTTaskCreateTaskingMessageResponse(
            TaskID=taskData.Task.ID,
            Success=result.Success,
            Error=result.Error,
            DisplayParams=f"{action} {port}",
        )
