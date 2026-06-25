#!/usr/bin/env python3
import mythic_container

import blueshell  # noqa: F401 - imports register payloads and commands


if __name__ == "__main__":
    mythic_container.mythic_service.start_and_run_forever()
