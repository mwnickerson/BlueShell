"""BlueShell stage payload builders."""

from __future__ import annotations

import pathlib

from mythic_container.MythicCommandBase import SupportedOS
from mythic_container.PayloadBuilder import (
    AgentType,
    BuildParameter,
    BuildParameterType,
    BuildResponse,
    BuildStatus,
    BuildStep,
    PayloadType,
)

from .build_support import run_agent_build, serialize_c2


class _BlueShellBase:
    """Shared metadata without registering an incomplete PayloadType."""
    author = "root"
    agent_type = AgentType.Agent
    mythic_encrypts = True
    supported_os = [SupportedOS.Windows]
    semver = "1.0.0"
    supports_dynamic_loading = True
    supports_multiple_c2_instances_in_build = True
    supports_multiple_c2_in_build = True
    c2_profiles = ["httpx", "http", "smb", "tcp"]
    agent_path = pathlib.Path(".") / "blueshell" / "mythic"
    agent_code_path = pathlib.Path(".") / "blueshell" / "agent_code"
    build_parameters = [
        BuildParameter(
            name="output_type",
            parameter_type=BuildParameterType.ChooseOne,
            choices=["shellcode", "raw", "exe", "service_exe", "dll"],
            default_value="shellcode",
            description="Requested payload artifact format",
        ),
        BuildParameter(
            name="architecture",
            parameter_type=BuildParameterType.ChooseOne,
            choices=["x64"],
            default_value="x64",
            description="Target architecture",
        ),
    ]
    build_steps = [
        BuildStep(
            step_name="Configuring",
            step_description="Serializing UUID, commands, and C2 configuration",
        ),
        BuildStep(
            step_name="Compiling",
            step_description="Building the selected release artifact",
        ),
    ]
    stage = ""

    async def build(self) -> BuildResponse:
        response = BuildResponse(status=BuildStatus.Error)
        output_type = self.get_parameter("output_type")
        try:
            config = {
                "payload_uuid": self.uuid,
                "stage": self.stage,
                "architecture": self.get_parameter("architecture"),
                "output_type": output_type,
                "commands": self.commands.get_commands(),
                "c2": serialize_c2(self.c2info),
                "crypto": "aes256_hmac_sha256",
            }
            result = run_agent_build(
                self.agent_code_path / self.stage,
                stage=self.stage,
                output_type=output_type,
                filename=getattr(self, "filename", self.name),
                config=config,
            )
            response.status = BuildStatus.Success
            response.payload = result.payload
            response.build_stdout = result.stdout
            response.build_stderr = result.stderr
            response.updated_filename = result.filename
            response.updated_command_list = self.commands.get_commands()
            response.build_message = f"Built {self.name} as {output_type}"
        except Exception as error:
            response.build_stderr = str(error)
            response.build_message = f"{self.name} build failed"
        return response


class BlueShellStage0(_BlueShellBase, PayloadType):
    name = "blueshell_stage0"
    file_extension = "bin"
    note = "BlueShell initial-stage payload."
    stage = "stage0"


class BlueShellStage1(_BlueShellBase, PayloadType):
    name = "blueshell_stage1"
    file_extension = "bin"
    note = "BlueShell full-feature payload."
    stage = "stage1"
