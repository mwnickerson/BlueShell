"""Register all payload types and commands with mythic_container."""

from .builder import BlueShellStage0, BlueShellStage1
from .cd import CdCommand
from .download import DownloadCommand
from .exit import ExitCommand
from .ls import LsCommand
from .pwd import PwdCommand
from .operator_commands import (
    CoffCommand,
    FingerprintCommand,
    KillCommand,
    PsCommand,
    RpfwdCommand,
    SocksCommand,
    Stage1Command,
)
from .shell import ShellCommand
from .sleep import SleepCommand
from .upload import UploadCommand

__all__ = [
    "BlueShellStage0",
    "BlueShellStage1",
    "CdCommand",
    "DownloadCommand",
    "ExitCommand",
    "LsCommand",
    "PwdCommand",
    "CoffCommand",
    "FingerprintCommand",
    "KillCommand",
    "PsCommand",
    "RpfwdCommand",
    "SocksCommand",
    "Stage1Command",
    "ShellCommand",
    "SleepCommand",
    "UploadCommand",
]
