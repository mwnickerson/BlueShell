"""Reusable command-definition helpers."""

from mythic_container.MythicCommandBase import (
    CommandAttributes,
    PTTaskCreateTaskingMessageResponse,
    PTTaskProcessResponseMessageResponse,
    SupportedOS,
)


WINDOWS_COMMAND = CommandAttributes(
    supported_os=[SupportedOS.Windows],
    suggested_command=True,
)


class StandardTasking:
    async def create_go_tasking(self, taskData):
        return PTTaskCreateTaskingMessageResponse(TaskID=taskData.Task.ID, Success=True)

    async def process_response(self, task, response):
        return PTTaskProcessResponseMessageResponse(TaskID=task.Task.ID, Success=True)
