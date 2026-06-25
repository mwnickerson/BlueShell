from mythic_container.MythicCommandBase import *

from .common import StandardTasking, WINDOWS_COMMAND


class SleepArguments(TaskArguments):
    def __init__(self, command_line, **kwargs):
        super().__init__(command_line, **kwargs)
        self.args = [
            CommandParameter(name="interval", type=ParameterType.Number),
            CommandParameter(
                name="jitter", type=ParameterType.Number, default_value=0
            ),
        ]

    async def parse_arguments(self):
        raw = self.command_line.strip()
        if raw.startswith("{"):
            self.load_args_from_json_string(raw)
            return
        values = raw.split()
        if not values:
            raise ValueError("an interval is required")
        self.add_arg("interval", int(values[0]))
        self.add_arg("jitter", int(values[1]) if len(values) > 1 else 0)


class SleepCommand(StandardTasking, CommandBase):
    cmd = "sleep"
    help_cmd = "sleep <seconds> [jitter]"
    description = "Update callback sleep interval and jitter."
    version = 1
    author = "root"
    argument_class = SleepArguments
    attributes = WINDOWS_COMMAND
