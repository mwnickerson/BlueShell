#!/usr/bin/env python3
import asyncio
import logging

import mythic_container
from mythic_container import PayloadBuilder, mythic_service

import blueshell  # noqa: F401 - imports register payloads and commands


async def start_blueshell():
    """Bind every payload RPC queue before advertising metadata to Mythic."""
    mythic_service.initialize()
    payload_types = []
    for payload_class in PayloadBuilder.PayloadType.__subclasses__():
        payload = payload_class()
        if not payload.name:
            continue
        if payload.name in PayloadBuilder.payloadTypes:
            raise RuntimeError(f"duplicate payload type: {payload.name}")
        PayloadBuilder.payloadTypes[payload.name] = payload
        payload_types.append(payload)

    expected = {"blueshell_stage0", "blueshell_stage1"}
    discovered = {payload.name for payload in payload_types}
    if discovered != expected:
        raise RuntimeError(
            f"payload discovery mismatch: expected {sorted(expected)}, "
            f"found {sorted(discovered)}"
        )

    for payload in payload_types:
        await mythic_service.startPayloadRabbitMQ(payload)

    # Consumer functions are scheduled as tasks. Yield long enough for their
    # queue declarations/bindings to execute before Mythic can send health RPCs.
    await asyncio.sleep(2)
    failed = [
        task
        for task in mythic_service.payloadQueueTasks
        if task.done() and not task.cancelled()
    ]
    if failed:
        errors = [repr(task.exception()) for task in failed]
        raise RuntimeError(f"payload queue consumers failed to start: {errors}")

    for payload in payload_types:
        await mythic_service.syncPayloadData(payload)

    logging.getLogger(__name__).critical(
        "BlueShell consumers ready: %s (%d queue tasks)",
        ", ".join(sorted(discovered)),
        len(mythic_service.payloadQueueTasks),
    )


def run_forever():
    loop = asyncio.get_event_loop()
    loop.run_until_complete(start_blueshell())
    loop.run_forever()


if __name__ == "__main__":
    run_forever()
