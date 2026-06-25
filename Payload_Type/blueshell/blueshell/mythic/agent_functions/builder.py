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


AGENT_PATH = pathlib.Path(".") / "blueshell" / "mythic"
AGENT_CODE_PATH = pathlib.Path(".") / "blueshell" / "agent_code"
C2_PROFILES = ["httpx", "http", "smb", "tcp"]


def build_parameters():
    # BuildParameter instances carry per-build values, so each payload type
    # needs its own list rather than sharing mutable instances.
    return [
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
        BuildParameter(
            name="debug",
            parameter_type=BuildParameterType.Boolean,
            default_value=False,
            description="Enable console diagnostics; never use for operational builds",
        ),
    ]


def build_steps():
    return [
        BuildStep(
            step_name="Configuring",
            step_description="Serializing UUID, commands, and C2 configuration",
        ),
        BuildStep(
            step_name="Compiling",
            step_description="Building the selected release artifact",
        ),
    ]


async def build_stage(payload: PayloadType, stage: str) -> BuildResponse:
    response = BuildResponse(status=BuildStatus.Error)
    output_type = payload.get_parameter("output_type")
    try:
        config = {
            "payload_uuid": payload.uuid,
            "stage": stage,
            "architecture": payload.get_parameter("architecture"),
            "output_type": output_type,
            "debug": bool(payload.get_parameter("debug")),
            "commands": payload.commands.get_commands(),
            "c2": serialize_c2(payload.c2info),
            "crypto": "aes256_hmac_sha256",
        }
        result = run_agent_build(
            payload.agent_code_path / stage,
            stage=stage,
            output_type=output_type,
            filename=getattr(payload, "filename", payload.name),
            config=config,
        )
        response.status = BuildStatus.Success
        response.payload = result.payload
        response.build_stdout = result.stdout
        response.build_stderr = result.stderr
        response.updated_filename = result.filename
        response.updated_command_list = payload.commands.get_commands()
        response.build_message = f"Built {payload.name} as {output_type}"
    except Exception as error:
        response.build_stderr = str(error)
        response.build_message = f"{payload.name} build failed"
    return response


class BlueShellStage0(PayloadType):
    name = "blueshell_stage0"
    file_extension = "bin"
    note = "BlueShell initial-stage payload."
    author = "root"
    agent_type = AgentType.Agent
    mythic_encrypts = True
    supported_os = [SupportedOS.Windows]
    semver = "1.0.0"
    supports_dynamic_loading = True
    supports_multiple_c2_instances_in_build = False
    supports_multiple_c2_in_build = False
    c2_profiles = C2_PROFILES
    agent_path = AGENT_PATH
    agent_code_path = AGENT_CODE_PATH
    build_parameters = build_parameters()
    build_steps = build_steps()

    async def build(self) -> BuildResponse:
        return await build_stage(self, "stage0")


class BlueShellStage1(PayloadType):
    name = "blueshell_stage1"
    file_extension = "bin"
    note = "BlueShell full-feature payload."
    author = "root"
    agent_type = AgentType.Agent
    mythic_encrypts = True
    supported_os = [SupportedOS.Windows]
    semver = "1.0.0"
    supports_dynamic_loading = True
    supports_multiple_c2_instances_in_build = False
    supports_multiple_c2_in_build = False
    c2_profiles = C2_PROFILES
    agent_path = AGENT_PATH
    agent_code_path = AGENT_CODE_PATH
    build_parameters = build_parameters()
    build_steps = build_steps()

    async def build(self) -> BuildResponse:
        return await build_stage(self, "stage1")
